//! V110-C69 `/spam <count> <message>`: the same line up to [`MAX`] times, as fast as Twitch
//! takes it. Each send goes through the normal path, so the pacing and duplicate dodging of
//! V108-C15 apply (mods, VIPs and the broadcaster go out at once, others about one a second).
//! `/spam stop` ends the one running in that chat (V110-F12: one at a time per chat, a line the
//! pacer still holds is dropped, leaving the chat ends it, and it gives up after about 3 s a line).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::Duration;

use tokio::time::Instant;

use super::{Place, RoomKey};

/// Most sends one `/spam` makes.
pub const MAX: u32 = 100;
/// Wait before trying again when Twitch's send limit is reached.
const RETRY: Duration = Duration::from_millis(500);
/// Longest wait for one send to fit under the limit before giving up.
const GIVE_UP: Duration = Duration::from_secs(60);

/// What follows `/spam`.
#[derive(Debug, PartialEq, Eq)]
pub enum Spam {
    Stop,
    Run { count: u32, text: String },
}

pub fn parse(args: &str) -> Result<Spam, String> {
    const USAGE: &str = "Usage: /spam 10 message, or /spam stop";
    let args = args.trim();
    if args.eq_ignore_ascii_case("stop") {
        return Ok(Spam::Stop);
    }
    let (n, text) = args.split_once(char::is_whitespace).ok_or(USAGE)?;
    let count: u32 = n.parse().map_err(|_| USAGE)?;
    if count == 0 {
        return Err(USAGE.into());
    }
    if count > MAX {
        return Err(format!("Up to {MAX}."));
    }
    let text = text.trim();
    if text.is_empty() {
        return Err(USAGE.into());
    }
    // Only words or /me: never a command run 100 times.
    if text.starts_with('/') && !text.starts_with("/me ") {
        return Err("Only text or /me can be spammed.".into());
    }
    Ok(Spam::Run { count, text: text.to_owned() })
}

/// The `/spam` running in each chat, with its stop flag. An entry lives as long as its run.
static RUNS: LazyLock<Mutex<HashMap<RoomKey, Arc<AtomicBool>>>> = LazyLock::new(Default::default);

fn runs() -> MutexGuard<'static, HashMap<RoomKey, Arc<AtomicBool>>> {
    RUNS.lock().unwrap_or_else(|p| p.into_inner())
}

/// A running `/spam`: its entry ends when this is dropped.
pub struct Claim {
    key: RoomKey,
    stopped: Arc<AtomicBool>,
}

impl Claim {
    fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        let mut m = runs();
        if m.get(&self.key).is_some_and(|f| Arc::ptr_eq(f, &self.stopped)) {
            m.remove(&self.key);
        }
    }
}

/// Starts a run in the chat, one at a time. Called before the task is spawned, so a
/// `/spam stop` right after can't miss it.
pub fn begin(key: &RoomKey) -> Result<Claim, String> {
    let mut m = runs();
    if m.contains_key(key) {
        return Err("Already spamming. /spam stop first.".into());
    }
    let stopped = Arc::new(AtomicBool::new(false));
    m.insert(key.clone(), stopped.clone());
    Ok(Claim { key: key.clone(), stopped })
}

/// Ends the run in that chat (also when the chat is left). False when none was running.
pub fn stop(key: &RoomKey) -> bool {
    let m = runs();
    let Some(f) = m.get(key) else { return false };
    f.store(true, Ordering::Relaxed);
    true
}

/// What the loop does next.
#[derive(Debug, PartialEq, Eq)]
enum Next {
    Send,
    Done,
    Stopped,
    TimedOut,
}

/// Total time one run may take: about 3 s a line.
fn deadline(count: u32) -> Duration {
    Duration::from_secs(3 * u64::from(count))
}

fn next(stopped: bool, sent: u32, count: u32, elapsed: Duration) -> Next {
    if stopped {
        Next::Stopped
    } else if sent >= count {
        Next::Done
    } else if elapsed > deadline(count) {
        Next::TimedOut
    } else {
        Next::Send
    }
}

/// The text of the send limit's error (`chat/send.rs` makes it: "Slow down: ...").
fn is_slow_down(e: &str) -> bool {
    e.starts_with("Slow down")
}

/// Whether a send held back by the limit may wait and try again: not past [`GIVE_UP` for this
/// line, nor past the run's [`deadline`].
fn may_retry(waited: Duration, elapsed: Duration, count: u32) -> bool {
    waited < GIVE_UP && elapsed <= deadline(count)
}

/// Sends `text` `count` times; the number that went out.
pub async fn run(
    app: &tauri::AppHandle,
    place: &Place,
    claim: Claim,
    count: u32,
    text: &str,
) -> Result<u32, String> {
    let begun = Instant::now();
    let mut sent = 0;
    loop {
        match next(claim.is_stopped(), sent, count, begun.elapsed()) {
            Next::Send => {}
            Next::Done | Next::Stopped => return Ok(sent),
            Next::TimedOut => return Err(format!("Stopped after {sent}: took too long.")),
        }
        let mut waited = Duration::ZERO;
        loop {
            if claim.is_stopped() {
                return Ok(sent);
            }
            let flag = claim.stopped.clone();
            let cancel: super::Cancel = Box::new(move || flag.load(Ordering::Relaxed));
            match super::send_until(app, place, text, cancel).await {
                Ok(()) => break,
                // A line the pacer still held when stop came is dropped, not an error.
                Err(_) if claim.is_stopped() => return Ok(sent),
                // V110-F80 The run's own deadline cuts the retries short too.
                Err(e) if is_slow_down(&e) && begun.elapsed() > deadline(count) => {
                    return Err(format!("Stopped after {sent}: took too long."));
                }
                Err(e) if is_slow_down(&e) && may_retry(waited, begun.elapsed(), count) => {
                    tokio::time::sleep(RETRY).await;
                    waited += RETRY;
                }
                Err(e) if sent > 0 => return Err(format!("Stopped after {sent}: {e}")),
                Err(e) => return Err(e),
            }
        }
        sent += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: &str) -> RoomKey {
        (crate::chat::Identity::Anon, c.to_owned())
    }

    #[test]
    fn parses_count_and_text() {
        assert_eq!(parse("3 hi there"), Ok(Spam::Run { count: 3, text: "hi there".into() }));
        assert_eq!(parse(" 100  /me waves "), Ok(Spam::Run { count: 100, text: "/me waves".into() }));
        assert_eq!(parse("STOP"), Ok(Spam::Stop));
        assert_eq!(parse("101 hi"), Err("Up to 100.".into()));
        assert!(parse("0 hi").is_err());
        assert!(parse("hi 3").is_err());
        assert!(parse("3").is_err());
        assert!(parse("3   ").is_err());
        assert!(parse("").is_err());
        assert_eq!(parse("5 /ban x"), Err("Only text or /me can be spammed.".into()));
    }

    #[test]
    fn one_run_per_chat_and_stop_reaches_it() {
        let k = key("spamtest_a");
        let r = begin(&k).unwrap();
        assert_eq!(begin(&k).err().as_deref(), Some("Already spamming. /spam stop first."));
        assert!(begin(&key("spamtest_b")).is_ok());
        assert!(!r.is_stopped());
        assert!(stop(&k));
        assert!(r.is_stopped());
        drop(r);
        // The entry goes with the run: nothing piles up, and a new run starts unstopped.
        assert!(!runs().contains_key(&k));
        let again = begin(&k).unwrap();
        assert!(!again.is_stopped());
        // Stopping a chat with no run leaves no entry.
        assert!(!stop(&key("spamtest_nobody")));
        assert!(!runs().contains_key(&key("spamtest_nobody")));
    }

    #[test]
    fn retries_stop_at_the_run_deadline() {
        // V110-F80: a held send used to retry for GIVE_UP (60 s) however short the run's deadline.
        let s = Duration::from_secs;
        assert!(may_retry(s(0), s(0), 1));
        assert!(may_retry(s(10), s(3), 1), "inside both");
        assert!(!may_retry(s(0), s(4), 1), "past the run's 3 s");
        assert!(!may_retry(GIVE_UP, s(1), 100), "past the line's limit");
        assert!(is_slow_down("Slow down: wait 3s."));
        assert!(is_slow_down("Slow down: too many waiting."));
        assert!(!is_slow_down("You can talk again in 3s."));
    }

    #[test]
    fn next_step_rules() {
        let s = Duration::from_secs;
        assert_eq!(next(false, 0, 5, s(0)), Next::Send);
        assert_eq!(next(false, 5, 5, s(1)), Next::Done);
        assert_eq!(next(true, 2, 5, s(1)), Next::Stopped);
        assert_eq!(next(false, 2, 5, s(15)), Next::Send);
        assert_eq!(next(false, 2, 5, s(16)), Next::TimedOut);
    }
}
