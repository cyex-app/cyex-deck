//! V107-P2 Paced calls to the third-party chat services (BTTV, FFZ, recent-messages):
//! a few in flight per service, a lane for the tab on screen, and retries on 429, 5xx
//! and network errors. With 200 tabs a request per channel at once got rate limited.
//! E1: FFZ background calls also start at most 8 a second, back off further on each
//! 429 and pause when FFZ says its bucket is nearly empty.
//! E2: 7TV's emote sets go through here too, at most 10 starts a second.

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
    /// E2 7TV's emote sets (v4 GQL).
    SevenTv,
}

/// How many services there are.
const N: usize = 4;

/// Who waits: the tab on screen (and one-off calls) go ahead of background tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lane {
    Front,
    #[default]
    Back,
}

/// In flight per service for background tabs: BTTV 4, FFZ 4, recent-messages 3
/// (a small third-party box behind nginx; it answered 503 under load), 7TV 4.
static BACK: [Semaphore; N] = [
    Semaphore::const_new(4),
    Semaphore::const_new(4),
    Semaphore::const_new(3),
    Semaphore::const_new(4),
];
/// Extra room for the front lane, so the shown tab never queues behind 200 others.
static FRONT: [Semaphore; N] = [
    Semaphore::const_new(2),
    Semaphore::const_new(2),
    Semaphore::const_new(2),
    Semaphore::const_new(2),
];
/// After a 429 the whole service waits until then, not just the one request.
static PAUSE: Mutex<[Option<Instant>; N]> = Mutex::new([None; N]);
/// E1 When the next background call of each service may start.
static NEXT: Mutex<[Option<Instant>; N]> = Mutex::new([None; N]);
/// E1 Least time between two background calls' starts: FFZ's bucket holds 120 per IP,
/// shared by every copy running. E2 7TV: at most 10 a second, so 200 tabs don't land at once.
const GAP_MS: [u64; N] = [0, 125, 0, 100];
/// E1 `ratelimit-remaining` below this pauses the service for a second.
const LOW: u64 = 10;

/// Waits before retry 1, 2 and 3 after a 5xx or a network error.
const BACKOFF: [u64; 3] = [1, 3, 8];
/// E1 Waits before each retry after a 429: a busy bucket needs more than another second.
const RATE_BACKOFF: [u64; 6] = [1, 2, 4, 8, 16, 30];
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

    /// E1 The call failed for now (429, 5xx, offline), not for good (404, no room, a bad
    /// answer): worth asking again later.
    pub fn transient(self, e: &Error) -> bool {
        e.is_network() || e.status().is_some_and(|s| self.retries(s))
    }

    /// Whether the whole service waits after `e`, not just the one call: a 429, and E2 a
    /// 7TV 503 (an HTML page from its nginx, down or busy: 200 tabs shouldn't keep knocking).
    fn pauses(self, e: &Error) -> bool {
        e.is_rate_limited() || (self == Service::SevenTv && e.status() == Some(503))
    }
}

impl Http {
    /// GET JSON from `service`, paced and retried (V107-P2, E1).
    pub async fn get_json_paced<T: DeserializeOwned>(&self, service: Service, lane: Lane, url: &str) -> Result<T> {
        json(service, lane, || self.0.get(url)).await
    }

    /// E2 POST JSON (a GQL read) to `service`, paced and retried like [`Self::get_json_paced`].
    pub async fn post_json_paced<T: DeserializeOwned>(
        &self,
        service: Service,
        lane: Lane,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        json(service, lane, || self.0.post(url).json(body)).await
    }
}

/// Like `http::json`, paced and retried. `build` makes the request again for each try.
/// Gives up after 3 retries, or 6 when rate limited.
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
            if lane == Lane::Back {
                wait_turn(service).await;
            }
            wait_pause(service).await;
            send(build()).await
        };
        if header.remaining.is_some_and(|r| r < LOW) {
            pause(service, Duration::from_secs(1));
        }
        let err = match res {
            Ok(body) => return serde_json::from_slice(&body).map_err(Error::Decode),
            Err(e) => e,
        };
        let Some(wait) = delay(service, &err, header.retry, attempt, unix_now()) else {
            return Err(err);
        };
        if service.pauses(&err) {
            pause(service, wait);
        }
        tokio::time::sleep(wait + jitter(wait)).await;
        attempt += 1;
    }
}

/// What an answer's headers said about the limit.
#[derive(Clone, Copy, Debug, Default)]
struct Limit {
    /// Retry-After / X-RateLimit-Reset.
    retry: Option<u64>,
    /// `ratelimit-remaining` (FFZ), `x-ratelimit-global-remaining` (7TV).
    remaining: Option<u64>,
}

impl Limit {
    /// E2 7TV sends no Retry-After, only when its window resets; that counts after a 429
    /// only (a 503 waits the backoff).
    fn read(status: u16, headers: &reqwest::header::HeaderMap) -> Self {
        let num = |k: &str| headers.get(k)?.to_str().ok()?.trim().parse::<u64>().ok();
        let reset = || (status == 429).then(|| num("x-ratelimit-global-reset")).flatten();
        Limit {
            retry: retry_secs(headers).or_else(reset),
            remaining: num("ratelimit-remaining").or_else(|| num("x-ratelimit-global-remaining")),
        }
    }
}

async fn send(req: RequestBuilder) -> (Result<Vec<u8>>, Limit) {
    let res = match req.send().await {
        Ok(r) => r,
        Err(e) => return (Err(Error::Network(e)), Limit::default()),
    };
    let status = res.status();
    let header = Limit::read(status.as_u16(), res.headers());
    let out = match res.bytes().await {
        Ok(b) if status.is_success() => Ok(b.to_vec()),
        Ok(b) => Err(Error::api(status.as_u16(), &b)),
        Err(e) => Err(Error::Network(e)),
    };
    (out, header)
}

/// How long to wait before retry `attempt` (0-based), or `None` to give up. `header`:
/// Retry-After / X-RateLimit-Reset (seconds, or a unix time), else the body's "wait N
/// seconds". A hint shorter than this attempt's backoff waits the backoff (E1: FFZ always
/// says one second, and with 200 tabs a second wasn't enough).
fn delay(service: Service, e: &Error, header: Option<u64>, attempt: usize, now: u64) -> Option<Duration> {
    if !service.transient(e) {
        return None;
    }
    let table: &[u64] = if e.is_rate_limited() { &RATE_BACKOFF } else { &BACKOFF };
    let base = *table.get(attempt)?;
    let hint = header
        .map(|h| if h > 1_000_000_000 { h.saturating_sub(now) } else { h })
        .or_else(|| e.api_message().and_then(wait_hint));
    let secs = hint.unwrap_or(0).max(base).clamp(1, MAX_WAIT);
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

/// E1 Takes the service's next start slot and waits for it.
async fn wait_turn(service: Service) {
    if let Some(at) = take_turn(service, Instant::now()) {
        tokio::time::sleep_until(at.into()).await;
    }
}

/// When a background call of `service` asked for at `now` may start (`None`: unpaced).
fn take_turn(service: Service, now: Instant) -> Option<Instant> {
    let gap = Duration::from_millis(GAP_MS[service.slot()]);
    if gap.is_zero() {
        return None;
    }
    let mut next = NEXT.lock().unwrap_or_else(|p| p.into_inner());
    let slot = &mut next[service.slot()];
    let at = slot.map_or(now, |n| n.max(now));
    *slot = Some(at + gap);
    Some(at)
}

async fn wait_pause(service: Service) {
    let until = PAUSE.lock().unwrap_or_else(|p| p.into_inner())[service.slot()];
    if let Some(until) = until.filter(|u| *u > Instant::now()) {
        tokio::time::sleep_until(until.into()).await;
    }
}

/// Up to half the wait plus 250 ms, so retries don't land together.
fn jitter(wait: Duration) -> Duration {
    let mut h = RandomState::new().build_hasher();
    h.write_u8(0);
    let max = wait.as_millis() as u64 / 2 + 250;
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
    fn rate_limits_back_off_longer() {
        let e = api(429, "x");
        let at: Vec<_> = (0..7).map(|n| secs(delay(Service::Ffz, &e, None, n, NOW))).collect();
        assert_eq!(at, [Some(1), Some(2), Some(4), Some(8), Some(16), Some(30), None]);
    }

    #[test]
    fn transient_or_final() {
        assert!(Service::Ffz.transient(&api(429, "x")));
        assert!(Service::Ffz.transient(&api(503, "x")));
        assert!(!Service::Ffz.transient(&api(404, "No such room")), "no room is final");
        assert!(!Service::Ffz.transient(&api(500, "x")), "FFZ's 500 means no room");
        assert!(Service::Bttv.transient(&api(500, "x")));
        assert!(!Service::Bttv.transient(&api(404, "x")));
    }

    #[test]
    fn background_starts_are_spaced() {
        let gap = Duration::from_millis(GAP_MS[Service::Ffz.slot()]);
        let now = Instant::now() + Duration::from_secs(3600);
        let a = take_turn(Service::Ffz, now).unwrap();
        let b = take_turn(Service::Ffz, now).unwrap();
        let c = take_turn(Service::Ffz, now).unwrap();
        assert!(a >= now);
        assert!(b - a >= gap && c - b >= gap);
        assert_eq!(take_turn(Service::Bttv, now), None, "BTTV is unpaced");
    }

    #[test]
    fn hints_win_over_backoff() {
        let ffz = api(
            429,
            r#"{"error":"Too Many Requests","status":429,"message":"You have exhausted your available requests. Please wait 1 seconds to continue."}"#,
        );
        assert_eq!(secs(delay(Service::Ffz, &ffz, None, 0, NOW)), Some(1));
        assert_eq!(secs(delay(Service::Ffz, &ffz, None, 2, NOW)), Some(4), "backoff over a short hint");
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
            assert!(jitter(Duration::from_secs(1)) < Duration::from_millis(750));
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

    #[test]
    fn seventv_retries_busy_answers_only() {
        let html = api(503, "<html><body>503</body></html>");
        assert!(Service::SevenTv.transient(&html));
        assert!(Service::SevenTv.transient(&api(429, "x")));
        assert!(Service::SevenTv.transient(&api(500, "x")), "7TV's 500 isn't an answer");
        assert!(!Service::SevenTv.transient(&api(404, "x")), "no 7TV user is final");
        assert!(!Service::SevenTv.transient(&api(400, "bad query")));
        let at = |n| secs(delay(Service::SevenTv, &html, None, n, NOW));
        assert_eq!((at(0), at(1), at(2), at(3)), (Some(1), Some(3), Some(8), None));
    }

    #[test]
    fn seventv_503_pauses_the_service() {
        assert!(Service::SevenTv.pauses(&api(503, "x")));
        assert!(Service::SevenTv.pauses(&api(429, "x")));
        assert!(!Service::SevenTv.pauses(&api(502, "x")));
        assert!(!Service::Bttv.pauses(&api(503, "x")), "only 7TV");
    }

    #[test]
    fn seventv_starts_are_spaced() {
        let gap = Duration::from_millis(GAP_MS[Service::SevenTv.slot()]);
        assert!(!gap.is_zero());
        let now = Instant::now() + Duration::from_secs(7200);
        let a = take_turn(Service::SevenTv, now).unwrap();
        let b = take_turn(Service::SevenTv, now).unwrap();
        assert!(b - a >= gap);
    }

    #[test]
    fn seventv_limit_headers() {
        use reqwest::header::{HeaderMap, HeaderValue};
        let mut h = HeaderMap::new();
        h.insert("x-ratelimit-global-remaining", HeaderValue::from_static("4"));
        h.insert("x-ratelimit-global-reset", HeaderValue::from_static("6"));
        let busy = Limit::read(429, &h);
        assert_eq!((busy.retry, busy.remaining), (Some(6), Some(4)));
        assert_eq!(Limit::read(503, &h).retry, None, "a 503 waits the backoff");
        h.insert("retry-after", HeaderValue::from_static("2"));
        assert_eq!(Limit::read(429, &h).retry, Some(2), "Retry-After first");
        assert_eq!(Limit::read(200, &HeaderMap::new()).remaining, None);
    }

    #[tokio::test]
    async fn seventv_posts_retry_a_503() {
        let base = serve(vec![
            "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/html\r\nContent-Length: 13\r\nConnection: close\r\n\r\n<html></html>",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"data\":{}}",
        ]);
        let http = Http::new().unwrap();
        let body = serde_json::json!({ "query": "query { x }" });
        let got: serde_json::Value = http
            .post_json_paced(Service::SevenTv, Lane::Front, &format!("{base}/v4/gql"), &body)
            .await
            .unwrap();
        assert_eq!(got["data"], serde_json::json!({}));
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
