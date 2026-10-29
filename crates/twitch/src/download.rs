//! V108-N02 Saving a file from a CDN link (a clip's MP4) to disk: streamed in chunks, with
//! progress, and stoppable. The partial file is `<name>.part` until it is whole.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use crate::Http;

/// How long a whole download may take: clips are small, a dead link shouldn't hang.
const LIMIT: Duration = Duration::from_secs(30 * 60);
/// How long no data may arrive before the download gives up.
const IDLE: Duration = Duration::from_secs(30);
/// How often a waiting read looks at the stop flag.
const TICK: Duration = Duration::from_millis(200);
/// Makes each `.part` name its own, so two saves to one path don't share a file.
static PARTS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    /// Offline, DNS, the link dropped mid-way.
    #[error("couldn’t reach Twitch")]
    Network(#[source] reqwest::Error),
    /// The server said no (an expired link is 403).
    #[error("the file isn’t there anymore ({0})")]
    Status(u16),
    /// Nothing came for too long.
    #[error("the connection went quiet")]
    Stalled,
    /// The file can't be written (full disk, no access).
    #[error("couldn’t write the file: {0}")]
    Disk(#[source] std::io::Error),
    /// The finished file can't take the place of the old one (open elsewhere, no access).
    #[error("couldn’t replace the file: {0}")]
    Replace(#[source] std::io::Error),
    /// The user stopped it.
    #[error("stopped")]
    Cancelled,
}

impl Http {
    /// Saves `url` to `dest`, calling `progress(done, total)` after each chunk. `cancel` set
    /// stops it (within a fraction of a second, even when no data is coming) and removes the
    /// partial file; 30 s without data ends it too. Returns the size in bytes.
    pub async fn download_to(
        &self,
        url: &str,
        dest: &Path,
        cancel: &AtomicBool,
        progress: impl FnMut(u64, Option<u64>),
    ) -> Result<u64, DownloadError> {
        self.download_paced(url, dest, cancel, progress, IDLE, TICK).await
    }

    async fn download_paced(
        &self,
        url: &str,
        dest: &Path,
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64, Option<u64>),
        idle: Duration,
        tick: Duration,
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
        let part = part_path(dest, PARTS.fetch_add(1, Ordering::Relaxed));
        let mut file = File::create(&part).map_err(DownloadError::Disk)?;
        let done = copy_chunks(&mut res, &mut file, total, cancel, &mut progress, idle, tick).await;
        drop(file);
        match done {
            Ok(n) => {
                // Replaces a file picked in the save dialog (the user agreed to overwrite).
                if let Err(e) = fs::rename(&part, dest) {
                    let _ = fs::remove_file(&part);
                    return Err(DownloadError::Replace(e));
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
    idle: Duration,
    tick: Duration,
) -> Result<u64, DownloadError> {
    let mut done = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        // The read waits for data, but looks at the stop flag every tick meanwhile.
        let next = tokio::time::timeout(idle, res.chunk());
        tokio::pin!(next);
        let read = loop {
            tokio::select! {
                r = &mut next => break r,
                () = tokio::time::sleep(tick) => {
                    if cancel.load(Ordering::Relaxed) {
                        return Err(DownloadError::Cancelled);
                    }
                }
            }
        };
        let Some(chunk) = read.map_err(|_| DownloadError::Stalled)?.map_err(DownloadError::Network)? else {
            break;
        };
        file.write_all(&chunk).map_err(DownloadError::Disk)?;
        done += chunk.len() as u64;
        progress(done, total);
    }
    file.flush().map_err(DownloadError::Disk)?;
    Ok(done)
}

/// `clip.mp4` -> `clip.mp4.<n>.part`.
fn part_path(dest: &Path, n: u64) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(format!(".{n}.part"));
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

    /// Like `serve`, but sends only 10 of the 100 promised bytes and then goes quiet.
    fn serve_stalling() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf);
                let _ = s.write_all(b"HTTP/1.1 200 OK
Content-Length: 100
Connection: close

0123456789");
                let _ = s.flush();
                std::thread::sleep(Duration::from_secs(5));
            }
        });
        format!("http://127.0.0.1:{port}/clip.mp4")
    }

    /// No `.part` file is left next to `dest`.
    fn no_parts(dest: &Path) -> bool {
        fs::read_dir(dest.parent().unwrap()).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".part"))
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
        assert!(no_parts(&dest));
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
        assert!(!dest.exists() && no_parts(&dest));
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
        assert!(!dest.exists() && no_parts(&dest));
    }

    #[tokio::test]
    async fn stop_works_while_no_data_comes() {
        let url = serve_stalling();
        let dest = temp("stop-stalled");
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            flag.store(true, Ordering::Relaxed);
        });
        let started = std::time::Instant::now();
        let err = Http::new()
            .unwrap()
            .download_paced(&url, &dest, &stop, |_, _| {}, Duration::from_secs(60), Duration::from_millis(50))
            .await
            .unwrap_err();
        assert!(matches!(err, DownloadError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(!dest.exists() && no_parts(&dest));
    }

    #[tokio::test]
    async fn a_dead_link_gives_up_after_the_idle_time() {
        let url = serve_stalling();
        let dest = temp("idle");
        let err = Http::new()
            .unwrap()
            .download_paced(&url, &dest, &AtomicBool::new(false), |_, _| {}, Duration::from_millis(300), Duration::from_millis(50))
            .await
            .unwrap_err();
        assert!(matches!(err, DownloadError::Stalled));
        assert!(!dest.exists() && no_parts(&dest));
    }

    #[tokio::test]
    async fn a_file_that_cannot_be_replaced_says_so() {
        let url = serve("200 OK", vec![1u8; 100]);
        let dest = temp("replace");
        fs::create_dir(&dest).unwrap();
        let err = Http::new()
            .unwrap()
            .download_to(&url, &dest, &AtomicBool::new(false), |_, _| {})
            .await
            .unwrap_err();
        assert!(matches!(err, DownloadError::Replace(_)));
        assert!(no_parts(&dest));
    }

    #[test]
    fn each_partial_file_has_its_own_name() {
        let a = part_path(Path::new("a/b.mp4"), 1);
        assert_eq!(a, PathBuf::from("a/b.mp4.1.part"));
        assert_ne!(a, part_path(Path::new("a/b.mp4"), 2));
    }
}
