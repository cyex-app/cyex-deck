//! V110-C69 `/spam <count> <message>`: the same line up to [`MAX`] times, as fast as Twitch
//! takes it. Each send goes through the normal path, so the pacing and duplicate dodging of
//! V108-C15 apply (mods, VIPs and the broadcaster go out at once, others about one a second).
//! `/spam stop` ends the ones running in that chat.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

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
        return Err(format!("{MAX} at most."));
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

/// Each chat's stop counter: a `/spam` ends when it moves.
static STOPS: LazyLock<Mutex<HashMap<RoomKey, u64>>> = LazyLock::new(Default::default);

fn stop_count(key: &RoomKey) -> u64 {
    STOPS.lock().map(|m| m.get(key).copied().unwrap_or(0)).unwrap_or(0)
}

pub fn stop(key: &RoomKey) {
    if let Ok(mut m) = STOPS.lock() {
        *m.entry(key.clone()).or_default() += 1;
    }
}

/// Sends `text` `count` times; the number that went out.
pub async fn run(app: &tauri::AppHandle, place: &Place, count: u32, text: &str) -> Result<u32, String> {
    let started = stop_count(&place.key);
    let mut sent = 0;
    while sent < count {
        let mut waited = Duration::ZERO;
        loop {
            if stop_count(&place.key) != started {
                return Ok(sent);
            }
            match super::send(app, place, text, None).await {
                Ok(()) => break,
                Err(e) if e.starts_with("Slow down") && waited < GIVE_UP => {
                    tokio::time::sleep(RETRY).await;
                    waited += RETRY;
                }
                Err(e) if sent > 0 => return Err(format!("Stopped after {sent}: {e}")),
                Err(e) => return Err(e),
            }
        }
        sent += 1;
    }
    Ok(sent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_count_and_text() {
        assert_eq!(parse("3 hi there"), Ok(Spam::Run { count: 3, text: "hi there".into() }));
        assert_eq!(parse(" 100  /me waves "), Ok(Spam::Run { count: 100, text: "/me waves".into() }));
        assert_eq!(parse("STOP"), Ok(Spam::Stop));
        assert_eq!(parse("101 hi"), Err("100 at most.".into()));
        assert!(parse("0 hi").is_err());
        assert!(parse("hi 3").is_err());
        assert!(parse("3").is_err());
        assert!(parse("3   ").is_err());
        assert!(parse("").is_err());
        assert_eq!(parse("5 /ban x"), Err("Only text or /me can be spammed.".into()));
    }
}
