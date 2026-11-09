//! V110-T05 Test builds only: the chat line injector (`tools/mock/chat.mjs`), named by
//! `CYEX_CHAT_MOCK=http://127.0.0.1:<port>` on a test data folder. Asked for lines every
//! 500 ms; each is a raw IRC line (PRIVMSG, CLEARCHAT, CLEARMSG, USERNOTICE, NOTICE) that goes
//! through the same parser and ingest path as the real ones, into every open room of its
//! channel. Nothing is ever sent to Twitch.

use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, Manager};

use crate::auth::Auth;

pub const VAR: &str = "CYEX_CHAT_MOCK";
const WHAT: &str = "chat";
const EVERY: Duration = Duration::from_millis(500);
/// What the injector may feed: incoming lines only.
const COMMANDS: [&str; 5] = ["PRIVMSG", "CLEARCHAT", "CLEARMSG", "USERNOTICE", "NOTICE"];

#[derive(Deserialize)]
struct Batch {
    next: u64,
    lines: Vec<String>,
}

/// The channel (login) of an injectable IRC line; `None` for anything else.
pub fn target(raw: &str) -> Option<String> {
    let line = cyex_chat::irc::Line::parse(raw)?;
    if !COMMANDS.contains(&line.command) {
        return None;
    }
    line.channel().filter(|c| !c.is_empty() && *c != "jtv").map(str::to_ascii_lowercase)
}

/// Starts asking the mock for lines (once, in `setup`).
pub fn start(app: &AppHandle) {
    let Ok(Some(base)) = crate::mock_url::gated(app, &[VAR], WHAT) else { return };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let http = app.state::<Auth>().http.clone();
        let mut after = 0u64;
        loop {
            tokio::time::sleep(EVERY).await;
            let Ok(batch) = http.get_json::<Batch>(&format!("{base}/_mock/lines?after={after}")).await else { continue };
            after = batch.next;
            for raw in batch.lines {
                if let Some(channel) = target(&raw) {
                    super::session::inject(&app, &channel, &raw);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t05_only_incoming_chat_lines_with_a_channel_are_fed() {
        let msg = "@display-name=A;id=1;room-id=1;tmi-sent-ts=1;user-id=2 :a!a@a.tmi.twitch.tv PRIVMSG #Chan :hi";
        assert_eq!(target(msg).as_deref(), Some("chan"));
        assert_eq!(
            target("@ban-duration=60;target-user-id=7 :tmi.twitch.tv CLEARCHAT #chan :u7").as_deref(),
            Some("chan")
        );
        assert_eq!(target("@login=a;target-msg-id=1 :tmi.twitch.tv CLEARMSG #chan :hi").as_deref(), Some("chan"));
        for refused in [
            ":tmi.twitch.tv NOTICE * :all chats",
            ":a!a@a.tmi.twitch.tv JOIN #chan",
            ":tmi.twitch.tv RECONNECT",
            "PING :x",
            "garbage",
            "",
        ] {
            assert_eq!(target(refused), None, "{refused}");
        }
    }

    #[test]
    fn t05_a_batch_decodes() {
        let b: Batch = serde_json::from_str(r#"{"next":3,"lines":["a","b"]}"#).unwrap();
        assert_eq!((b.next, b.lines.len()), (3, 2));
    }

    #[test]
    fn t05_gated_like_every_other_mock() {
        assert_eq!(crate::mock_url::pick(None, false, WHAT), Ok(None));
        assert!(crate::mock_url::pick(Some("http://127.0.0.1:5260"), false, WHAT).is_err(), "real data: refused");
        assert!(crate::mock_url::pick(Some("https://twitch.tv"), true, WHAT).is_err());
        assert!(crate::mock_url::pick(Some("http://127.0.0.1:5260"), true, WHAT).unwrap().is_some());
    }
}
