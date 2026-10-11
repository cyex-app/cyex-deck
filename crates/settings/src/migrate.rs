//! V102-02 The app was renamed. On the first start under the new name, the old name's
//! folder is copied over once: every settings partition, the 3-day backups, `.env`,
//! plugins and the rest. The old folder is left as it was.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// What `copy_once` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Copied {
    /// The new folder was made from the old one.
    Yes,
    /// The new folder was there already.
    AlreadyThere,
    /// There's no old folder to copy.
    NothingOld,
}

/// Copies `old` to `new` when `new` doesn't exist and `old` does. Files are copied as they
/// are, never opened as text (`.env` holds secrets). Folders named in `skip` stay behind,
/// e.g. browser caches. The copy is made next to `new` and renamed into place at the end,
/// so a copy cut short leaves nothing and runs again on the next start.
pub fn copy_once(old: &Path, new: &Path, skip: &[&str]) -> io::Result<Copied> {
    if new.exists() {
        return Ok(Copied::AlreadyThere);
    }
    if !old.is_dir() {
        return Ok(Copied::NothingOld);
    }
    let tmp = staging(new);
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    if let Err(e) = copy_dir(old, &tmp, skip) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    fs::rename(&tmp, new)?;
    Ok(Copied::Yes)
}

/// `cyex-deck` → `cyex-deck.copying`, in the same parent folder.
fn staging(new: &Path) -> PathBuf {
    let mut name = new.file_name().unwrap_or_default().to_os_string();
    name.push(".copying");
    new.with_file_name(name)
}

fn copy_dir(from: &Path, to: &Path, skip: &[&str]) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name();
        let target = to.join(&name);
        if kind.is_dir() {
            if !skip.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                copy_dir(&entry.path(), &target, skip)?;
            }
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn put(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn copies_partitions_backups_and_env_and_leaves_the_old_folder() {
        let t = TempDir::new("migrate-copy");
        let (old, new) = (t.path().join("old"), t.path().join("new"));
        put(&old.join("settings/accounts.json"), b"{\"list\":[]}");
        put(&old.join("settings/backups/2026-10-10/tabs.json"), b"{}");
        put(&old.join(".env"), b"TWITCH_CLIENT_ID=x\r\n\xff");
        fs::create_dir_all(old.join("plugins")).unwrap();

        assert_eq!(copy_once(&old, &new, &[]).unwrap(), Copied::Yes);
        assert_eq!(
            fs::read(new.join("settings/accounts.json")).unwrap(),
            b"{\"list\":[]}"
        );
        assert!(new.join("settings/backups/2026-10-10/tabs.json").is_file());
        // Byte for byte, even what isn't text.
        assert_eq!(fs::read(new.join(".env")).unwrap(), b"TWITCH_CLIENT_ID=x\r\n\xff");
        assert!(new.join("plugins").is_dir());
        // The old folder is untouched and nothing is left half made.
        assert_eq!(fs::read(old.join(".env")).unwrap(), b"TWITCH_CLIENT_ID=x\r\n\xff");
        assert!(old.join("settings/backups/2026-10-10/tabs.json").is_file());
        assert!(!staging(&new).exists());
    }

    #[test]
    fn runs_once() {
        let t = TempDir::new("migrate-once");
        let (old, new) = (t.path().join("old"), t.path().join("new"));
        put(&old.join("settings/chat.json"), b"old");
        assert_eq!(copy_once(&old, &new, &[]).unwrap(), Copied::Yes);
        put(&new.join("settings/chat.json"), b"changed since");
        assert_eq!(copy_once(&old, &new, &[]).unwrap(), Copied::AlreadyThere);
        assert_eq!(
            fs::read(new.join("settings/chat.json")).unwrap(),
            b"changed since"
        );
    }

    #[test]
    fn nothing_old_makes_nothing() {
        let t = TempDir::new("migrate-none");
        let (old, new) = (t.path().join("old"), t.path().join("new"));
        assert_eq!(copy_once(&old, &new, &[]).unwrap(), Copied::NothingOld);
        assert!(!new.exists());
    }

    #[test]
    fn skips_caches_and_redoes_a_copy_cut_short() {
        let t = TempDir::new("migrate-skip");
        let (old, new) = (t.path().join("old"), t.path().join("new"));
        put(&old.join("twitch/1/EBWebView/Default/Cache/data_0"), b"cache");
        put(&old.join("twitch/1/EBWebView/Default/Network/Cookies"), b"jar");
        // Left over from a start that was cut off mid-copy.
        put(&staging(&new).join("twitch/half"), b"x");
        assert_eq!(copy_once(&old, &new, &["Cache"]).unwrap(), Copied::Yes);
        assert!(new.join("twitch/1/EBWebView/Default/Network/Cookies").is_file());
        assert!(!new.join("twitch/1/EBWebView/Default/Cache").exists());
        assert!(!new.join("twitch/half").exists());
    }
}
