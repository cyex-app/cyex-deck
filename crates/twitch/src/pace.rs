//! V107-P2 Paced calls to the third-party chat services (BTTV, FFZ, recent-messages):
//! a few in flight per service, a lane for the tab on screen, and retries on 429, 5xx
//! and network errors. With 200 tabs a request per channel at once got rate limited.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use tokio::sync::Semaphore;

use crate::http::retry_secs;
use crate::{Error, Http, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    Bttv,
    Ffz,
    Recent,
}

/// Who waits: the tab on screen (and one-off calls) go ahead of background tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lane {
    Front,
    #[default]
    Back,
}

/// In flight per service for background tabs: BTTV 4, FFZ 4, recent-messages 3
/// (a small third-party box behind nginx; it answered 503 under load).
static BACK: [Semaphore; 3] = [Semaphore::const_new(4), Semaphore::const_new(4), Semaphore::const_new(3)];
/// Extra room for the front lane, so the shown tab never queues behind 200 others.
static FRONT: [Semaphore; 3] = [Semaphore::const_new(2), Semaphore::const_new(2), Semaphore::const_new(2)];
/// After a 429 the whole service waits until then, not just the one request.
static PAUSE: Mutex<[Option<Instant>; 3]> = Mutex::new([None; 3]);

/// Waits before retry 1, 2 and 3 when the service gives no hint.
const BACKOFF: [u64; 3] = [1, 3, 8];
/// A hint longer than this is cut to it.
const MAX_WAIT: u64 = 30;

impl Service {
    fn slot(self) -> usize {
        self as usize
    }

    /// FFZ answers 500 for a channel without a room: that's an answer, not a hiccup.
    fn retries(self, status: u16) -> bool {
        status == 429 || (status >= 500 && !(self == Service::Ffz && status == 500))
    }
}

impl Http {
    /// GET JSON from `service`, paced and retried (V107-P2). Gives up after 3 retries.
    pub async fn get_json_paced<T: DeserializeOwned>(&self, service: Service, lane: Lane, url: &str) -> Result<T> {
        json(service, lane, || self.0.get(url)).await
    }
}

/// Like `http::json`, paced and retried. `build` makes the request again for each try.
pub(crate) async fn json<T: DeserializeOwned>(
    service: Service,
    lane: Lane,
    build: impl Fn() -> RequestBuilder,
) -> Result<T> {
    let sem = match lane {
        Lane::Front => &FRONT[service.slot()],
        Lane::Back => &BACK[service.slot()],
    };
    let mut attempt = 0;
    loop {
        let (res, header) = {
            // Never closed, so acquiring can't fail.
            let _permit = sem.acquire().await.ok();
            wait_pause(service).await;
            send(build()).await
        };
        let err = match res {
            Ok(body) => return serde_json::from_slice(&body).map_err(Error::Decode),
            Err(e) => e,
        };
        let Some(wait) = delay(service, &err, header, attempt, unix_now()) else {
            return Err(err);
        };
        if err.is_rate_limited() {
            pause(service, wait);
        }
        tokio::time::sleep(wait + jitter(wait)).await;
        attempt += 1;
    }
}

async fn send(req: RequestBuilder) -> (Result<Vec<u8>>, Option<u64>) {
    let res = match req.send().await {
        Ok(r) => r,
        Err(e) => return (Err(Error::Network(e)), None),
    };
    let status = res.status();
    let header = retry_secs(res.headers());
    let out = match res.bytes().await {
        Ok(b) if status.is_success() => Ok(b.to_vec()),
        Ok(b) => Err(Error::api(status.as_u16(), &b)),
        Err(e) => Err(Error::Network(e)),
    };
    (out, header)
}

/// How long to wait before retry `attempt` (0-based), or `None` to give up. `header`:
/// Retry-After / X-RateLimit-Reset (seconds, or a unix time). Else the body's "wait N
/// seconds", else `BACKOFF`.
fn delay(service: Service, e: &Error, header: Option<u64>, attempt: usize, now: u64) -> Option<Duration> {
    let retry = e.is_network() || e.status().is_some_and(|s| service.retries(s));
    if !retry || attempt >= BACKOFF.len() {
        return None;
    }
    let hint = header
        .map(|h| if h > 1_000_000_000 { h.saturating_sub(now) } else { h })
        .or_else(|| e.api_message().and_then(wait_hint));
    let secs = hint.unwrap_or(BACKOFF[attempt]).clamp(1, MAX_WAIT);
    Some(Duration::from_secs(secs))
}

/// FFZ: "Please wait 1 seconds to continue."
fn wait_hint(message: &str) -> Option<u64> {
    let lower = message.to_ascii_lowercase();
    let rest = &lower[lower.find("wait")? + 4..];
    let rest = rest.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let n: u64 = digits.parse().ok()?;
    rest[digits.len()..].trim_start().starts_with('s').then_some(n)
}

fn pause(service: Service, wait: Duration) {
    let until = Instant::now() + wait;
    let mut p = PAUSE.lock().unwrap_or_else(|p| p.into_inner());
    let slot = &mut p[service.slot()];
    *slot = Some(slot.map_or(until, |u| u.max(until)));
}

async fn wait_pause(service: Service) {
    let until = PAUSE.lock().unwrap_or_else(|p| p.into_inner())[service.slot()];
    if let Some(until) = until.filter(|u| *u > Instant::now()) {
        tokio::time::sleep_until(until.into()).await;
    }
}

/// Up to a fifth of the wait plus 250 ms, so retries don't land together.
fn jitter(wait: Duration) -> Duration {
    let mut h = RandomState::new().build_hasher();
    h.write_u8(0);
    let max = wait.as_millis() as u64 / 5 + 250;
    Duration::from_millis(h.finish() % max)
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use super::*;

    const NOW: u64 = 1_760_000_000;

    fn api(status: u16, body: &str) -> Error {
        Error::api(status, body.as_bytes())
    }

    fn secs(d: Option<Duration>) -> Option<u64> {
        d.map(|d| d.as_secs())
    }

    #[test]
    fn which_errors_retry() {
        let d = |s, e: &Error| secs(delay(s, e, None, 0, NOW));
        assert_eq!(d(Service::Bttv, &api(429, "slow down")), Some(1));
        assert_eq!(d(Service::Recent, &api(503, "<html>503</html>")), Some(1));
        assert_eq!(d(Service::Bttv, &api(500, "x")), Some(1));
        assert_eq!(d(Service::Bttv, &api(404, r#"{"message":"user not found"}"#)), None);
        assert_eq!(d(Service::Ffz, &api(500, "no room")), None, "FFZ's 500 means no room");
        assert_eq!(d(Service::Ffz, &api(502, "bad gateway")), Some(1));
        assert_eq!(d(Service::Recent, &api(403, "x")), None);
        let ignored = Error::Api { status: 200, message: "channel_ignored: x".into() };
        assert_eq!(d(Service::Recent, &ignored), None);
        let decode = Error::Decode(serde_json::from_str::<u8>("x").unwrap_err());
        assert_eq!(d(Service::Bttv, &decode), None);
    }

    #[test]
    fn backoff_then_give_up() {
        let e = api(503, "x");
        let at = |n| secs(delay(Service::Recent, &e, None, n, NOW));
        assert_eq!((at(0), at(1), at(2), at(3)), (Some(1), Some(3), Some(8), None));
    }

    #[test]
    fn hints_win_over_backoff() {
        let ffz = api(
            429,
            r#"{"error":"Too Many Requests","status":429,"message":"You have exhausted your available requests. Please wait 1 seconds to continue."}"#,
        );
        assert_eq!(secs(delay(Service::Ffz, &ffz, None, 2, NOW)), Some(1));
        let e = api(429, "x");
        assert_eq!(secs(delay(Service::Bttv, &e, Some(5), 0, NOW)), Some(5), "Retry-After");
        assert_eq!(secs(delay(Service::Bttv, &e, Some(NOW + 7), 0, NOW)), Some(7), "reset as a unix time");
        assert_eq!(secs(delay(Service::Bttv, &e, Some(NOW - 3), 0, NOW)), Some(1), "already past");
        assert_eq!(secs(delay(Service::Bttv, &e, Some(3600), 0, NOW)), Some(MAX_WAIT));
    }

    #[test]
    fn wait_hints() {
        assert_eq!(wait_hint("Please wait 12 seconds to continue."), Some(12));
        assert_eq!(wait_hint("Wait 2s"), Some(2));
        assert_eq!(wait_hint("wait for it"), None);
        assert_eq!(wait_hint("wait 5 minutes"), None);
    }

    #[test]
    fn jitter_stays_small() {
        for _ in 0..50 {
            assert!(jitter(Duration::from_secs(1)) < Duration::from_millis(450));
        }
    }

    /// Answers each connection with the next canned reply.
    fn serve(replies: Vec<&'static str>) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        std::thread::spawn(move || {
            for r in replies {
                let (mut s, _) = l.accept().unwrap();
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf);
                s.write_all(r.as_bytes()).unwrap();
            }
        });
        base
    }

    #[tokio::test]
    async fn retries_a_429_then_reads_the_answer() {
        let base = serve(vec![
            "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]",
        ]);
        let http = Http::new().unwrap();
        let got: Vec<u8> = http.get_json_paced(Service::Recent, Lane::Front, &format!("{base}/x")).await.unwrap();
        assert!(got.is_empty());
    }

    #[tokio::test]
    async fn a_404_is_not_retried() {
        let base = serve(vec![
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ]);
        let http = Http::new().unwrap();
        let got = http.get_json_paced::<Vec<u8>>(Service::Bttv, Lane::Front, &format!("{base}/x")).await;
        assert_eq!(got.unwrap_err().status(), Some(404));
    }
}
