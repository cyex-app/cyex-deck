//! V107-R41 Bulk dashboard writes (clips and videos delete, publish, redemption status):
//! one at a time, at most one request a second, a throttled or failing request sent again
//! after a longer wait, and a Stop (PERF-01). What went through comes back, with the reason
//! the rest didn't (CORR-02).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use cyex_twitch::Error;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

/// Between two requests, retries included.
pub const PACE: Duration = Duration::from_secs(1);
/// Waits before each retry of a throttled or failing request.
const BACKOFF: [Duration; 3] = [
    Duration::from_secs(2),
    Duration::from_secs(5),
    Duration::from_secs(10),
];
/// GQL payload codes that mean "too fast".
const THROTTLED: [&str; 5] = [
    "RATE_LIMITED",
    "RATELIMITED",
    "REQUESTS_THROTTLED",
    "THROTTLED",
    "TOO_MANY_REQUESTS",
];

/// What a bulk write did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmBulk")]
pub struct Bulk {
    /// Ids (or slugs) Twitch confirmed.
    pub done: Vec<String>,
    /// Why the rest didn't go, as a line.
    pub error: Option<String>,
    /// Stop was pressed.
    pub stopped: bool,
}

/// One bulk write at a time; Stop ends the running one.
#[derive(Default)]
pub struct Gate {
    running: AtomicBool,
    stop: AtomicBool,
}

struct Running<'a>(&'a Gate);

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::SeqCst);
    }
}

impl Gate {
    fn enter(&self) -> Result<Running<'_>, String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("Another change is still running. Wait or stop it.".into());
        }
        self.stop.store(false, Ordering::SeqCst);
        Ok(Running(self))
    }
}

/// Stops the running bulk write after its current request.
pub fn stop(app: &AppHandle) {
    if let Some(g) = app.try_state::<Gate>() {
        g.stop.store(true, Ordering::SeqCst);
    }
}

/// A request worth sending again.
fn transient(e: &Error) -> bool {
    e.is_rate_limited()
        || e.is_server_error()
        || e.is_network()
        || matches!(e, Error::Refused(c) if THROTTLED.contains(&c.as_str()))
}

/// Runs `send` over `items` in parts of `size`, through the app's gate.
pub async fn run<Fut>(
    app: &AppHandle,
    items: Vec<String>,
    size: usize,
    say: impl Fn(&Error) -> String,
    send: impl FnMut(Vec<String>) -> Fut,
) -> Result<Bulk, String>
where
    Fut: Future<Output = cyex_twitch::Result<Vec<String>>>,
{
    let gate = app.try_state::<Gate>().ok_or("Not ready yet.")?;
    let _running = gate.enter()?;
    Ok(drive(
        unique(items),
        size,
        &gate.stop,
        |d| tokio::time::sleep(d),
        say,
        send,
    )
    .await)
}

/// No blanks, no repeats, first order kept.
pub fn unique(mut v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    v.retain(|s| !s.trim().is_empty() && seen.insert(s.clone()));
    v
}

/// The loop, with its waits and Stop handed in (tests).
async fn drive<W, Fut>(
    items: Vec<String>,
    size: usize,
    stop: &AtomicBool,
    mut wait: impl FnMut(Duration) -> W,
    say: impl Fn(&Error) -> String,
    mut send: impl FnMut(Vec<String>) -> Fut,
) -> Bulk
where
    W: Future<Output = ()>,
    Fut: Future<Output = cyex_twitch::Result<Vec<String>>>,
{
    let mut out = Bulk::default();
    let mut first = true;
    for part in items.chunks(size.max(1)) {
        let mut tries = 0;
        loop {
            if stop.load(Ordering::SeqCst) {
                out.stopped = true;
                return out;
            }
            if !first {
                wait(if tries == 0 {
                    PACE
                } else {
                    BACKOFF[tries - 1].max(PACE)
                })
                .await;
            }
            first = false;
            match send(part.to_vec()).await {
                Ok(done) => {
                    out.done.extend(done);
                    break;
                }
                Err(e) if transient(&e) && tries < BACKOFF.len() => tries += 1,
                Err(e) => {
                    out.error = Some(say(&e));
                    return out;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    fn say(e: &Error) -> String {
        format!("no: {e}")
    }

    #[tokio::test]
    async fn v107_r41_paced_one_a_second_and_retried_when_throttled() {
        let waits = RefCell::new(Vec::new());
        let calls = RefCell::new(0);
        let stop = AtomicBool::new(false);
        let items: Vec<String> = (0..5).map(|i| i.to_string()).collect();
        let out = drive(
            items,
            2,
            &stop,
            |d| {
                waits.borrow_mut().push(d);
                async {}
            },
            say,
            |part: Vec<String>| {
                *calls.borrow_mut() += 1;
                let n = *calls.borrow();
                async move {
                    match n {
                        2 => Err(Error::Refused("RATE_LIMITED".into())),
                        3 => Err(Error::Api {
                            status: 503,
                            message: "busy".into(),
                        }),
                        _ => Ok(part),
                    }
                }
            },
        )
        .await;
        assert_eq!(
            out,
            Bulk {
                done: (0..5).map(|i| i.to_string()).collect(),
                error: None,
                stopped: false
            }
        );
        assert_eq!(*waits.borrow(), [PACE, BACKOFF[0], BACKOFF[1], PACE]);
        assert!(
            waits.borrow().iter().all(|w| *w >= PACE),
            "never faster than one a second"
        );
    }

    #[tokio::test]
    async fn v107_r41_a_refusal_stops_and_keeps_the_reason() {
        let stop = AtomicBool::new(false);
        let calls = RefCell::new(0);
        let out = drive(
            vec!["a".into(), "b".into(), "c".into()],
            1,
            &stop,
            |_| async {},
            say,
            |part: Vec<String>| {
                *calls.borrow_mut() += 1;
                let n = *calls.borrow();
                async move {
                    if n == 2 {
                        Err(Error::Refused("FORBIDDEN".into()))
                    } else {
                        Ok(part)
                    }
                }
            },
        )
        .await;
        assert_eq!(out.done, ["a"]);
        assert_eq!(out.error.as_deref(), Some("no: Twitch said no: FORBIDDEN"));
        assert_eq!(*calls.borrow(), 2);
    }

    #[tokio::test]
    async fn v107_r41_retries_give_up_and_stop_stops() {
        let stop = AtomicBool::new(false);
        let calls = RefCell::new(0);
        let out = drive(
            vec!["a".into()],
            1,
            &stop,
            |_| async {},
            say,
            |_| {
                *calls.borrow_mut() += 1;
                async {
                    Err(Error::Api {
                        status: 429,
                        message: "slow".into(),
                    })
                }
            },
        )
        .await;
        assert_eq!(*calls.borrow(), 1 + BACKOFF.len());
        assert!(out.error.is_some() && out.done.is_empty());
        let stop = AtomicBool::new(true);
        let out = drive(
            vec!["a".into()],
            1,
            &stop,
            |_| async {},
            say,
            |p: Vec<String>| async move { Ok(p) },
        )
        .await;
        assert_eq!(
            out,
            Bulk {
                stopped: true,
                ..Bulk::default()
            }
        );
    }

    #[test]
    fn v107_r41_one_bulk_at_a_time() {
        let g = Gate::default();
        let held = g.enter().unwrap();
        assert!(g.enter().is_err());
        drop(held);
        g.stop.store(true, Ordering::SeqCst);
        let _again = g.enter().unwrap();
        assert!(!g.stop.load(Ordering::SeqCst), "a new run clears Stop");
    }

    #[test]
    fn v107_r41_unique_drops_blanks_and_repeats() {
        assert_eq!(
            unique(vec!["a".into(), " ".into(), "b".into(), "a".into()]),
            ["a", "b"]
        );
    }
}
