//! V108-N02 Saving a file from a CDN link (a clip's MP4) to disk: streamed in chunks, with
//! progress, and stoppable. The partial file is `<name>.part` until it is whole.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::Http;

/// How long a whole download may take: clips are small, a dead link shouldn't hang.
const LIMIT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    /// Offline, DNS, the link dropped mid-way.
    #[error("couldn’t reach Twitch")]
    Network(#[source] reqwest::Error),
    /// The server said no (an expired link is 403).
    #[error("the file isn’t there anymore ({0})")]
    Status(u16),
    /// The file can't be written (full disk, no access).
    #[error("couldn’t write the file: {0}")]
    Disk(#[source] std::io::Error),
    /// The user stopped it.
    #[error("stopped")]
    Cancelled,
}

impl Http {
    /// Saves `url` to `dest`, calling `progress(done, total)` after each chunk. `cancel` set
    /// stops it between chunks and removes the partial file. Returns the size in bytes.
    pub async fn download_to(
        &self,
        url: &str,
        dest: &Path,
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Result<u64, DownloadError> {
        let mut res = self
            .0
            .get(url)
            .timeout(LIMIT)
            .send()
            .await
            .map_err(DownloadError::Network)?;
        let status = res.status();
        if !status.is_success() {
            return Err(DownloadError::Status(status.as_u16()));
        }
        let total = res.content_length().filter(|n| *n > 0);
        let part = part_path(dest);
        let mut file = File::create(&part).map_err(DownloadError::Disk)?;
        let done = copy_chunks(&mut res, &mut file, total, cancel, &mut progress).await;
        drop(file);
        match done {
            Ok(n) => {
                // Replaces a file picked in the save dialog (the user agreed to overwrite).
                if let Err(e) = fs::rename(&part, dest) {
                    let _ = fs::remove_file(&part);
                    return Err(DownloadError::Disk(e));
                }
                Ok(n)
            }
            Err(e) => {
                let _ = fs::remove_file(&part);
                Err(e)
            }
        }
    }
}

async fn copy_chunks(
    res: &mut reqwest::Response,
    file: &mut File,
    total: Option<u64>,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(u64, Option<u64>),
) -> Result<u64, DownloadError> {
    let mut done = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        let Some(chunk) = res.chunk().await.map_err(DownloadError::Network)? else {
            break;
        };
        file.write_all(&chunk).map_err(DownloadError::Disk)?;
        done += chunk.len() as u64;
        progress(done, total);
    }
    file.flush().map_err(DownloadError::Disk)?;
    Ok(done)
}

/// `clip.mp4` -> `clip.mp4.part`.
fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::AtomicBool;

    use super::*;

    /// A one-shot HTTP server on localhost answering with `status` and `body`.
    fn serve(status: &'static str, body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf);
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(&body);
            }
        });
        format!("http://127.0.0.1:{port}/clip.mp4")
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cyex-download-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("clip.mp4")
    }

    #[tokio::test]
    async fn saves_the_file_and_reports_progress() {
        let body: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
        let url = serve("200 OK", body.clone());
        let dest = temp("ok");
        let mut last = (0, None);
        let n = Http::new()
            .unwrap()
            .download_to(&url, &dest, &AtomicBool::new(false), |d, t| last = (d, t))
            .await
            .unwrap();
        assert_eq!(n, 50_000);
        assert_eq!(last, (50_000, Some(50_000)));
        assert_eq!(fs::read(&dest).unwrap(), body);
        assert!(!part_path(&dest).exists());
    }

    #[tokio::test]
    async fn an_expired_link_leaves_no_file() {
        let url = serve("403 Forbidden", b"no".to_vec());
        let dest = temp("403");
        let err = Http::new()
            .unwrap()
            .download_to(&url, &dest, &AtomicBool::new(false), |_, _| {})
            .await
            .unwrap_err();
        assert!(matches!(err, DownloadError::Status(403)));
        assert!(!dest.exists() && !part_path(&dest).exists());
    }

    #[tokio::test]
    async fn stopping_removes_the_partial_file() {
        let url = serve("200 OK", vec![7u8; 10_000]);
        let dest = temp("stop");
        let err = Http::new()
            .unwrap()
            .download_to(&url, &dest, &AtomicBool::new(true), |_, _| {})
            .await
            .unwrap_err();
        assert!(matches!(err, DownloadError::Cancelled));
        assert!(!dest.exists() && !part_path(&dest).exists());
    }

    #[test]
    fn the_partial_file_sits_next_to_it() {
        assert_eq!(part_path(Path::new("a/b.mp4")), PathBuf::from("a/b.mp4.part"));
    }
}
