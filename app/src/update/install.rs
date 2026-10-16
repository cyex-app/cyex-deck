//! V105-20 Downloads the installer, checks it against the release's SHA-256 and starts it
//! quietly. A file that doesn't match is deleted and never started.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use cyex_core::version;

use super::Updater;
use crate::busy::Busy;
use crate::auth::Auth;

use super::sign::NO_MATCH;

const DOWNLOAD_FAILED: &str = "Download failed. Try again later.";
const NO_START: &str = "Couldn’t start the installer.";

#[derive(Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum UpdateProgress {
    Downloading { percent: u8 },
    Verifying,
    Restarting,
}

pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("cyex-deck-update")
}

pub async fn run(app: AppHandle, on: Channel<UpdateProgress>) -> Result<(), String> {
    let updater = app.state::<Updater>();
    let _busy = Busy::take(&updater.busy).ok_or("Wait for the download to finish.")?;
    let update = super::available(&app).ok_or(DOWNLOAD_FAILED)?;
    // The installer must be this release's own, and the release newer than the running app.
    if !update.installer.as_ref().is_some_and(|i| installable(&i.name, &update.version, &version::current())) {
        return Err(DOWNLOAD_FAILED.into());
    }
    let (Some(installer), Some(sha_url)) = (update.installer, update.sha256_url) else {
        return Err(DOWNLOAD_FAILED.into());
    };
    // V105-20 No `.sig` in the release: don't even download.
    let Some(sig_url) = update.sig_url else {
        return Err(super::sign::NOT_SIGNED.into());
    };
    // The name comes from GitHub: a plain file name or nothing.
    if installer.name.contains(['/', '\\', ':']) {
        return Err(DOWNLOAD_FAILED.into());
    }
    let http = app.state::<Auth>().http.clone();
    let dir = temp_dir();
    fs::create_dir_all(&dir).map_err(|_| DOWNLOAD_FAILED)?;
    let path = dir.join(&installer.name);
    let mut last = u8::MAX;
    let got = http
        .download(&installer.url, &path, installer.size, |done, total| {
            let percent = (done * 100).checked_div(total).unwrap_or(0).min(100) as u8;
            if percent != last {
                last = percent;
                let _ = on.send(UpdateProgress::Downloading { percent });
            }
        })
        .await;
    if got.is_err() {
        let _ = fs::remove_file(&path);
        return Err(DOWNLOAD_FAILED.into());
    }
    let _ = on.send(UpdateProgress::Verifying);
    let hash = match http.get_text(&sha_url, &[], 1024).await {
        Ok(text) => match sha_line(&text) {
            Some(h) => h,
            None => {
                let _ = fs::remove_file(&path);
                return Err(NO_MATCH.into());
            }
        },
        Err(_) => {
            let _ = fs::remove_file(&path);
            return Err(DOWNLOAD_FAILED.into());
        }
    };
    let check_path = path.clone();
    checked(&path, tauri::async_runtime::spawn_blocking(move || verify(&check_path, &hash)).await)?;
    // V105-20 Signed with the author's key, or it doesn't run. The `.sig` is small text.
    let sig = match http.get_text(&sig_url, &[], 2048).await {
        Ok(text) => text,
        Err(_) => {
            let _ = fs::remove_file(&path);
            return Err(DOWNLOAD_FAILED.into());
        }
    };
    let (sig_path, name) = (path.clone(), installer.name.clone());
    checked(
        &path,
        tauri::async_runtime::spawn_blocking(move || super::sign::verify(&sig_path, &name, Some(&sig))).await,
    )?;
    crate::settings::flush_now(&app);
    let _ = on.send(UpdateProgress::Restarting);
    std::process::Command::new(&path)
        .args(["/P", "/UPDATE", "/R"])
        .spawn()
        .map_err(|_| NO_START)?;
    app.exit(0);
    Ok(())
}

/// The release is newer than `current` and `name` is `cyex-deck_<version>_x64-setup.exe` for
/// exactly that version (an older signed installer on a newer tag is a downgrade).
fn installable(name: &str, release: &str, current: &version::Version) -> bool {
    let Some(want) = version::parse(release).filter(|v| v > current) else {
        return false;
    };
    name.strip_prefix("cyex-deck_")
        .and_then(|n| n.strip_suffix("_x64-setup.exe"))
        .and_then(version::parse)
        == Some(want)
}

/// A check that ran on the blocking pool. If the task itself failed (a panic), the file is
/// deleted too: nothing unchecked stays on disk.
fn checked<E>(path: &Path, joined: Result<Result<(), String>, E>) -> Result<(), String> {
    joined.unwrap_or_else(|_| {
        let _ = fs::remove_file(path);
        Err(NO_MATCH.into())
    })
}

/// The hash from a `.sha256` file: its first word, lowercase, 64 hex digits.
pub fn sha_line(text: &str) -> Option<String> {
    let word = text.split_whitespace().next()?.to_ascii_lowercase();
    (word.len() == 64 && word.bytes().all(|b| b.is_ascii_hexdigit())).then_some(word)
}

/// Hashes the file and compares; a file that doesn't match is deleted.
pub fn verify(path: &Path, want: &str) -> Result<(), String> {
    let got = hash_file(path).map_err(|_| NO_MATCH.to_owned());
    match got {
        Ok(h) if h == want => Ok(()),
        _ => {
            let _ = fs::remove_file(path);
            Err(NO_MATCH.into())
        }
    }
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn only_the_releases_own_newer_installer() {
        let cur = version::parse("0.1.4").unwrap();
        assert!(installable("cyex-deck_0.1.5_x64-setup.exe", "0.1.5", &cur));
        assert!(!installable("cyex-deck_0.1.2_x64-setup.exe", "0.1.5", &cur));
        assert!(!installable("cyex-deck_0.1.4_x64-setup.exe", "0.1.4", &cur));
        assert!(!installable("cyex-deck_0.1.3_x64-setup.exe", "0.1.3", &cur));
        assert!(!installable("other.exe", "0.1.5", &cur));
    }

    #[test]
    fn hash_line() {
        let line = format!("{}  cyex-deck_0.1.5_x64-setup.exe\n", ABC.to_ascii_uppercase());
        assert_eq!(sha_line(&line).as_deref(), Some(ABC));
        assert_eq!(sha_line(ABC).as_deref(), Some(ABC));
        for junk in ["", "  \n", "<html>404</html>", "abc", &ABC[..63]] {
            assert!(sha_line(junk).is_none(), "{junk}");
        }
    }

    #[test]
    fn verify_matches_and_deletes_on_mismatch() {
        let path = std::env::temp_dir().join(format!("cyex-verify-test-{}.bin", std::process::id()));
        fs::write(&path, b"abc").unwrap();
        assert!(verify(&path, ABC).is_ok());
        assert!(path.exists());
        let wrong = "0".repeat(64);
        assert_eq!(verify(&path, &wrong), Err(NO_MATCH.to_owned()));
        assert!(!path.exists());
        // A file that isn't there doesn't match either.
        assert!(verify(&path, ABC).is_err());
    }
}
