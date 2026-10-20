//! V107-P2 Paced calls to the third-party chat services (BTTV, FFZ, recent-messages):
//! a few in flight per service, a lane for the tab on screen, and retries on 429, 5xx
//! and network errors. With 200 tabs a request per channel at once got rate limited.
//! E1: FFZ background calls also start at most 8 a second, back off further on each
//! 429 and pause when FFZ says its bucket is nearly empty.
//! E2: 7TV's emote sets go through here too, at most 10 starts a second.
//! S1 One [`Pacer`] per service: its permits, and one gate that hands out start times
//! with pauses and spacing together (B2, B3).

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

/// Who waits: the tab on screen (and one-off calls) go ahead of background tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lane {
    Front,
    #[default]
    Back,
}

/// How one call is paced and retried.
#[derive(Clone, Copy, Debug, Default)]
pub struct Opts {
    pub lane: Lane,
    /// S2 The caller asks again later on its own (the emote retry queue): a background
    /// call gives up after one retry on a 429 and at once on a 5xx. The pause still goes up.
    pub quick: bool,
    /// B7 Statuses that are this call's answer, never retried (FFZ's 500 for "no room").
    pub answers: &'static [u16],
}

impl From<Lane> for Opts {
    fn from(lane: Lane) -> Self {
        Opts {
            lane,
            ..Opts::default()
        }
    }
}

/// One service's permits and start gate.
struct Pacer {
    /// In flight for background tabs.
    back: Semaphore,
    /// Extra room for the front lane, so the shown tab never queues behind 200 others.
    front: Semaphore,
    gate: Mutex<Gate>,
    /// E1 Least time between two background calls' starts.
    gap: Duration,
}

/// When calls may start.
#[derive(Clone, Copy, Debug, Default)]
struct Gate {
    /// E1 When the next background call may start.
    next: Option<Instant>,
    /// After a 429 the whole service waits until then, not just the one request.
    paused_until: Option<Instant>,
}

impl Pacer {
    const fn new(back: usize, gap_ms: u64) -> Self {
        Pacer {
            back: Semaphore::const_new(back),
            front: Semaphore::const_new(2),
            gate: Mutex::new(Gate {
                next: None,
                paused_until: None,
            }),
            gap: Duration::from_millis(gap_ms),
        }
    }

    fn gate(&self) -> std::sync::MutexGuard<'_, Gate> {
        self.gate.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Waits until a call in `lane` may start: past any pause (also one that grew while
    /// waiting) and, in the background, on its own spaced slot.
    async fn admit(&self, lane: Lane) {
        let mut mine = None;
        loop {
            let Some(at) = self.gate().admit(Instant::now(), lane, self.gap, mine) else {
                return;
            };
            mine = Some(at);
            tokio::time::sleep_until(at.into()).await;
        }
    }

    fn pause(&self, wait: Duration) {
        self.gate().pause(Instant::now() + wait);
    }
}

impl Gate {
    /// `None`: start now. Else the reserved start to sleep until, then ask again with it
    /// as `mine`. A slot comes after any pause; background slots are `gap` apart.
    fn admit(&mut self, now: Instant, lane: Lane, gap: Duration, mine: Option<Instant>) -> Option<Instant> {
        let paused = self.paused_until.filter(|p| *p > now);
        if mine.is_some() && paused.is_none() {
            return None;
        }
        let mut at = paused.unwrap_or(now);
        if lane == Lane::Back && !gap.is_zero() {
            at = at.max(self.next.unwrap_or(now));
            self.next = Some(at + gap);
        }
        (at > now).then_some(at)
    }

    fn pause(&mut self, until: Instant) {
        self.paused_until = Some(self.paused_until.map_or(until, |u| u.max(until)));
    }
}

/// BTTV 4 in flight, FFZ 4 (and E1 8 starts a second: its bucket holds 120 per IP,
/// shared by every copy running), recent-messages 3 (a small box behind nginx; it
/// answered 503 under load), E2 7TV 4 at most 10 starts a second.
static PACERS: [Pacer; 4] = [
    Pacer::new(4, 0),
    Pacer::new(4, 125),
    Pacer::new(3, 0),
    Pacer::new(4, 100),
];

/// E1 `ratelimit-remaining` below this pauses the service for a second.
const LOW: u64 = 10;

/// Waits before retry 1, 2 and 3 after a 5xx or a network error.
const BACKOFF: [u64; 3] = [1, 3, 8];
/// E1 Waits before each retry after a 429: a busy bucket needs more than another second.
const RATE_BACKOFF: [u64; 6] = [1, 2, 4, 8, 16, 30];
/// A hint longer than this is cut to it.
const MAX_WAIT: u64 = 30;

impl Service {
    fn pacer(self) -> &'static Pacer {
        &PACERS[self as usize]
    }

    fn retries(status: u16) -> bool {
        status == 429 || status >= 500
    }

    /// E1 The call failed for now (429, 5xx, offline), not for good (404, no room, a bad
    /// answer): worth asking again later.
    pub fn transient(self, e: &Error) -> bool {
        e.is_network() || e.status().is_some_and(Self::retries)
    }

    /// Whether the whole service waits after `e`, not just the one call: a 429, and E2 a
    /// 7TV 503 (an HTML page from its nginx, down or busy: 200 tabs shouldn't keep knocking).
    fn pauses(self, e: &Error) -> bool {
        e.is_rate_limited() || (self == Service::SevenTv && e.status() == Some(503))
    }
}

impl Opts {
    /// S2 Retries this call makes after `e` at most.
    fn max_retries(&self, e: &Error) -> usize {
        match (self.quick && self.lane == Lane::Back, e.is_rate_limited()) {
            (true, true) => 1,
            (true, false) => 0,
            (false, true) => RATE_BACKOFF.len(),
            (false, false) => BACKOFF.len(),
        }
    }
}

impl Http {
    /// GET JSON from `service`, paced and retried (V107-P2, E1).
    pub async fn get_json_paced<T: DeserializeOwned>(&self, service: Service, lane: Lane, url: &str) -> Result<T> {
        json(service, lane, || self.0.get(url)).await
    }

    /// Like [`Self::get_json_paced`], with S2/B7 options.
    pub async fn get_json_paced_with<T: DeserializeOwned>(&self, service: Service, opts: Opts, url: &str) -> Result<T> {
        json(service, opts, || self.0.get(url)).await
    }

    /// E2 POST JSON (a GQL read) to `service`, paced and retried like [`Self::get_json_paced`].
    pub async fn post_json_paced<T: DeserializeOwned>(
        &self,
        service: Service,
        opts: impl Into<Opts>,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        json(service, opts, || self.0.post(url).json(body)).await
    }
}

/// Like `http::json`, paced and retried. `build` makes the request again for each try.
/// Gives up after 3 retries, or 6 when rate limited (fewer with `quick`, S2).
pub(crate) async fn json<T: DeserializeOwned>(
    service: Service,
    opts: impl Into<Opts>,
    build: impl Fn() -> RequestBuilder,
) -> Result<T> {
    let opts = opts.into();
    let pacer = service.pacer();
    let sem = match opts.lane {
        Lane::Front => &pacer.front,
        Lane::Back => &pacer.back,
    };
    let mut attempt = 0;
    loop {
        let (res, header) = {
            // Never closed, so acquiring can't fail.
            let _permit = sem.acquire().await.ok();
            pacer.admit(opts.lane).await;
            send(build()).await
        };
        if header.remaining.is_some_and(|r| r < LOW) {
            pacer.pause(Duration::from_secs(1));
        }
        let err = match res {
            Ok(body) => return serde_json::from_slice(&body).map_err(Error::Decode),
            Err(e) => e,
        };
        let Some(wait) = delay(service, &err, header.retry, attempt, unix_now(), opts.answers) else {
            return Err(err);
        };
        if service.pauses(&err) {
            pacer.pause(wait);
        }
        if attempt >= opts.max_retries(&err) {
            return Err(err);
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
/// says one second, and with 200 tabs a second wasn't enough). `answers`: never retried.
fn delay(
    service: Service,
    e: &Error,
    header: Option<u64>,
    attempt: usize,
    now: u64,
    answers: &[u16],
) -> Option<Duration> {
    if !service.transient(e) || e.status().is_some_and(|s| answers.contains(&s)) {
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
        let d = |s, e: &Error| secs(delay(s, e, None, 0, NOW, &[]));
        assert_eq!(d(Service::Bttv, &api(429, "slow down")), Some(1));
        assert_eq!(d(Service::Recent, &api(503, "<html>503</html>")), Some(1));
        assert_eq!(d(Service::Bttv, &api(500, "x")), Some(1));
        assert_eq!(d(Service::Bttv, &api(404, r#"{"message":"user not found"}"#)), None);
        assert_eq!(d(Service::Ffz, &api(500, "x")), Some(1), "B7 a 500 on globals or badges is retried");
        let room = delay(Service::Ffz, &api(500, "no room"), None, 0, NOW, &[500]);
        assert_eq!(room, None, "B7 only the room call takes FFZ's 500 as no room");
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
        let at = |n| secs(delay(Service::Recent, &e, None, n, NOW, &[]));
        assert_eq!((at(0), at(1), at(2), at(3)), (Some(1), Some(3), Some(8), None));
    }

    #[test]
    fn rate_limits_back_off_longer() {
        let e = api(429, "x");
        let at: Vec<_> = (0..7).map(|n| secs(delay(Service::Ffz, &e, None, n, NOW, &[]))).collect();
        assert_eq!(at, [Some(1), Some(2), Some(4), Some(8), Some(16), Some(30), None]);
    }

    #[test]
    fn transient_or_final() {
        assert!(Service::Ffz.transient(&api(429, "x")));
        assert!(Service::Ffz.transient(&api(503, "x")));
        assert!(!Service::Ffz.transient(&api(404, "No such room")), "no room is final");
        assert!(Service::Ffz.transient(&api(500, "x")), "B7 not final for every FFZ call");
        assert!(Service::Bttv.transient(&api(500, "x")));
        assert!(!Service::Bttv.transient(&api(404, "x")));
    }

    const GAP: Duration = Duration::from_millis(125);

    #[test]
    fn background_starts_are_spaced() {
        let now = Instant::now();
        let mut g = Gate::default();
        assert_eq!(g.admit(now, Lane::Back, GAP, None), None, "the first goes at once");
        let b = g.admit(now, Lane::Back, GAP, None).unwrap();
        let c = g.admit(now, Lane::Back, GAP, None).unwrap();
        assert!(b - now >= GAP && c - b >= GAP);
        assert_eq!(g.admit(now, Lane::Front, GAP, None), None, "the front lane isn't spaced");
        assert_eq!(Gate::default().admit(now, Lane::Back, Duration::ZERO, None), None, "BTTV is unpaced");
        assert!(PACERS[Service::Ffz as usize].gap >= GAP);
        assert!(PACERS[Service::Bttv as usize].gap.is_zero());
    }

    /// B3 After a pause the waiting calls start one gap apart, not all at once.
    #[test]
    fn b3_calls_after_a_pause_stay_spaced() {
        let now = Instant::now();
        let end = now + Duration::from_secs(4);
        let mut g = Gate::default();
        g.pause(end);
        let slots: Vec<Instant> = (0..4).map(|_| g.admit(now, Lane::Back, GAP, None).unwrap()).collect();
        assert_eq!(slots[0], end);
        assert!(slots.windows(2).all(|w| w[1] - w[0] >= GAP));
        // Each wakes at its slot and goes.
        for s in &slots {
            assert_eq!(g.admit(*s, Lane::Back, GAP, Some(*s)), None);
        }
        // The front lane waits the pause too.
        let mut g = Gate::default();
        g.pause(end);
        assert_eq!(g.admit(now, Lane::Front, GAP, None), Some(end));
    }

    /// B2 A pause that grew while a call slept on it is waited out too.
    #[test]
    fn b2_a_pause_extended_while_waiting_is_honored() {
        let now = Instant::now();
        let first = now + Duration::from_secs(1);
        let longer = now + Duration::from_secs(5);
        let mut g = Gate::default();
        g.pause(first);
        let at = g.admit(now, Lane::Front, Duration::ZERO, None).unwrap();
        assert_eq!(at, first);
        g.pause(longer);
        g.pause(first);
        assert_eq!(g.admit(at, Lane::Front, Duration::ZERO, Some(at)), Some(longer), "a shorter pause never shortens it");
        assert_eq!(g.admit(longer, Lane::Front, Duration::ZERO, Some(longer)), None);
        // In the background it takes a new spaced slot after the longer pause.
        let mut g = Gate::default();
        g.pause(first);
        let a = g.admit(now, Lane::Back, GAP, None).unwrap();
        g.pause(longer);
        assert!(g.admit(a, Lane::Back, GAP, Some(a)).unwrap() >= longer);
    }

    #[tokio::test]
    async fn b2_admit_sleeps_through_a_longer_pause() {
        static P: Pacer = Pacer::new(1, 0);
        P.pause(Duration::from_millis(60));
        let start = Instant::now();
        let waiter = tokio::spawn(async {
            P.admit(Lane::Front).await;
            Instant::now()
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        P.pause(Duration::from_millis(200));
        let went = waiter.await.unwrap();
        assert!(went - start >= Duration::from_millis(200), "went after {:?}", went - start);
    }

    /// S2 Background calls with their own retry queue give up early; the pause still goes up.
    #[test]
    fn s2_quick_background_calls_retry_less() {
        let quick = Opts {
            lane: Lane::Back,
            quick: true,
            answers: &[],
        };
        assert_eq!(quick.max_retries(&api(429, "x")), 1);
        assert_eq!(quick.max_retries(&api(503, "x")), 0);
        let front = Opts { lane: Lane::Front, ..quick };
        assert_eq!(front.max_retries(&api(503, "x")), BACKOFF.len(), "the tab on screen keeps its retries");
        assert_eq!(Opts::from(Lane::Back).max_retries(&api(429, "x")), RATE_BACKOFF.len());
    }

    #[test]
    fn hints_win_over_backoff() {
        let ffz = api(
            429,
            r#"{"error":"Too Many Requests","status":429,"message":"You have exhausted your available requests. Please wait 1 seconds to continue."}"#,
        );
        assert_eq!(secs(delay(Service::Ffz, &ffz, None, 0, NOW, &[])), Some(1));
        assert_eq!(secs(delay(Service::Ffz, &ffz, None, 2, NOW, &[])), Some(4), "backoff over a short hint");
        let e = api(429, "x");
        assert_eq!(secs(delay(Service::Bttv, &e, Some(5), 0, NOW, &[])), Some(5), "Retry-After");
        assert_eq!(secs(delay(Service::Bttv, &e, Some(NOW + 7), 0, NOW, &[])), Some(7), "reset as a unix time");
        assert_eq!(secs(delay(Service::Bttv, &e, Some(NOW - 3), 0, NOW, &[])), Some(1), "already past");
        assert_eq!(secs(delay(Service::Bttv, &e, Some(3600), 0, NOW, &[])), Some(MAX_WAIT));
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
        let at = |n| secs(delay(Service::SevenTv, &html, None, n, NOW, &[]));
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

    /// B7 The room call's 500 is its answer: back at once, not after three retries.
    #[tokio::test]
    async fn b7_an_answer_status_is_not_retried() {
        let base = serve(vec![
            "HTTP/1.1 500 Internal Server Error
Content-Length: 0
Connection: close

",
        ]);
        let http = Http::new().unwrap();
        let opts = Opts {
            lane: Lane::Front,
            quick: false,
            answers: &[500],
        };
        let start = Instant::now();
        let got = http.get_json_paced_with::<Vec<u8>>(Service::Ffz, opts, &format!("{base}/room")).await;
        assert_eq!(got.unwrap_err().status(), Some(500));
        assert!(start.elapsed() < Duration::from_millis(900), "no retry wait");
    }
}
