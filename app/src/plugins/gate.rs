//! V107-Mb What a plugin's command returns writes to Twitch (a chat line, a ban, a whisper,
//! a `/chain` of them) only when the plugin holds SendMessages, within its send limit
//! (the one `send_message` uses). Without it, one notice per plugin per session, then quiet.

use std::collections::BTreeSet;
use std::sync::Mutex;

use cyex_plugins::Plugin;
use tauri::AppHandle;

/// Plugins already told this session, by folder name.
static TOLD: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// What becomes of one write a plugin's output asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Go,
    /// Refused; say this once.
    Notice(String),
    /// Refused, already said.
    Quiet,
    /// Refused: its send limit is used up.
    TooMany(String),
}

/// The plugins (`ids`, outermost first) behind a line that writes to Twitch. `Ok(true)`: go
/// on. `Ok(false)`: refused quietly. `Err`: refused, with what to show.
pub fn check(app: &AppHandle, ids: &[String]) -> Result<bool, String> {
    for id in ids {
        let p = super::get(app, id);
        let name = super::name_of(app, id);
        match decide(p.as_deref(), &name, |id| first_time(&TOLD, id)) {
            Verdict::Go => {}
            Verdict::Quiet => return Ok(false),
            Verdict::Notice(e) | Verdict::TooMany(e) => return Err(e),
        }
    }
    Ok(true)
}

/// One plugin's say. `None`: it stopped meanwhile, so it holds nothing.
fn decide(p: Option<&Plugin>, name: &str, first: impl FnOnce(&str) -> bool) -> Verdict {
    match p {
        Some(p) if p.granted.send => verdict(true, || p.take_send(), name, || first(&p.id)),
        Some(p) => verdict(false, || false, name, || first(&p.id)),
        None => verdict(false, || false, name, || first(name)),
    }
}

/// The pure rule: no SendMessages, no write; with it, only while the limit has room.
fn verdict(send: bool, take: impl FnOnce() -> bool, name: &str, first: impl FnOnce() -> bool) -> Verdict {
    if !send {
        return if first() {
            Verdict::Notice(format!("{name} needs Send messages to do that."))
        } else {
            Verdict::Quiet
        };
    }
    if take() {
        Verdict::Go
    } else {
        Verdict::TooMany(format!(
            "{name} can send {} messages in {} seconds.",
            cyex_plugins::SEND_LIMIT,
            cyex_plugins::SEND_WINDOW.as_secs()
        ))
    }
}

/// `true` the first time `id` asks.
fn first_time(told: &Mutex<BTreeSet<String>>, id: &str) -> bool {
    told.lock().unwrap_or_else(|p| p.into_inner()).insert(id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_send_messages_nothing_writes() {
        let told = Mutex::new(BTreeSet::new());
        let ask = |send: bool, room: bool| verdict(send, || room, "Hi", || first_time(&told, "hi"));
        assert_eq!(ask(false, true), Verdict::Notice("Hi needs Send messages to do that.".into()));
        // Once per plugin per session, then quiet.
        assert_eq!(ask(false, true), Verdict::Quiet);
        assert!(first_time(&told, "other"));
    }

    #[test]
    fn with_send_messages_the_limit_counts() {
        let first = || panic!("no notice with SendMessages");
        assert_eq!(verdict(true, || true, "Hi", first), Verdict::Go);
        assert_eq!(
            verdict(true, || false, "Hi", first),
            Verdict::TooMany("Hi can send 10 messages in 30 seconds.".into())
        );
    }

    #[test]
    fn a_stopped_plugin_holds_nothing() {
        assert!(matches!(decide(None, "Gone", |_| true), Verdict::Notice(_)));
        assert_eq!(decide(None, "Gone", |_| false), Verdict::Quiet);
    }
}
