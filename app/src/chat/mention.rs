//! U6, 2.6, V104-01 Highlights and mentions that asked for it flash the taskbar, show a
//! Windows pop-up, or play the sound (the chime, or a mention's own file; V1051-01 in Rust).
//! V106-25 One message plays one sound, however many chats show it. V107-H16 That holds
//! across the firehose and joined chats too: one message id, one sound, however many
//! rules, tabs or feeds it matched.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cyex_chat::model::{ChatMessage, plain};
use cyex_chat::rules::Outcome;
use tauri::AppHandle;

use crate::toasts::{self, Open, Pop};

/// A pop-up keeps this much of the message.
const PREVIEW_CHARS: usize = 120;
/// V107-H16 A message id that played stays known this long: the same line from the
/// firehose and an open chat (two tabs, two accounts) arrives within seconds.
const WINDOW: Duration = Duration::from_secs(30);
/// At most this many ids, however busy.
const KEEP: usize = 256;
type Recent = Mutex<VecDeque<(String, Instant)>>;
static SOUNDED: Recent = Mutex::new(VecDeque::new());
static POPPED: Recent = Mutex::new(VecDeque::new());

/// `global`: from logs.cyex.app's firehose (V104-02); its pop-up opens the `#channel`
/// window (V102-05) instead of a tab. V106-25 No sound from a channel that is only in muted tabs.
/// Each message sounds once and pops up once.
pub fn notify(app: &AppHandle, channel: &str, m: &ChatMessage, outcome: &Outcome, global: bool) {
    if outcome.flash {
        toasts::flash(app);
    }
    let from = || m.sender.as_ref().map(|s| s.display.clone()).unwrap_or_default();
    // R9 One sound and one pop-up per message; flash dedups through `FLASHING`. V107-H16 An
    // id is only taken by what actually played, so a silent match can't swallow a sound.
    if outcome.sound && !crate::tabs::mute::hushes_login(app, channel) && first(&SOUNDED, &m.id) {
        crate::sound::alert(app, outcome.sound_file.as_deref());
    }
    if outcome.notify && !toasts::app_in_front(app) && first(&POPPED, &m.id) {
        let text: String = plain(&m.body).chars().take(PREVIEW_CHARS).collect();
        let login = channel.to_owned();
        toasts::show(
            app,
            Pop {
                title: if global {
                    format!("{} · #{channel} · global", from())
                } else {
                    format!("{} · #{channel}", from())
                },
                text,
                avatar: None,
                open: if global { Open::Room { login } } else { Open::Channel { login } },
            },
        );
    }
}

/// False when this message already played.
fn first(recent: &Recent, id: &str) -> bool {
    remember(&mut recent.lock().unwrap_or_else(|p| p.into_inner()), id, Instant::now())
}

/// Adds `id`; false when it was seen within `WINDOW`. No id: always new.
fn remember(recent: &mut VecDeque<(String, Instant)>, id: &str, now: Instant) -> bool {
    if id.is_empty() {
        return true;
    }
    while recent.front().is_some_and(|(_, t)| now.saturating_duration_since(*t) > WINDOW) {
        recent.pop_front();
    }
    if recent.iter().any(|(r, _)| r == id) {
        return false;
    }
    if recent.len() >= KEEP {
        recent.pop_front();
    }
    recent.push_back((id.to_owned(), now));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // V107-H16 Firehose, a joined chat and a second tab all hand over the same id: one sound.
    #[test]
    fn a_message_seen_twice_sounds_once() {
        let (mut recent, t) = (VecDeque::new(), Instant::now());
        assert!(remember(&mut recent, "m1", t), "firehose");
        assert!(!remember(&mut recent, "m1", t + Duration::from_secs(2)), "joined chat");
        assert!(!remember(&mut recent, "m1", t + Duration::from_secs(3)), "second tab");
        assert!(remember(&mut recent, "m2", t + Duration::from_secs(3)));
    }

    #[test]
    fn old_ids_drop_out_and_empty_ids_always_sound() {
        let (mut recent, t) = (VecDeque::new(), Instant::now());
        assert!(remember(&mut recent, "m1", t));
        assert!(remember(&mut recent, "m1", t + WINDOW + Duration::from_secs(1)), "window passed");
        assert!(remember(&mut recent, "", t));
        assert!(remember(&mut recent, "", t));
        for i in 0..KEEP {
            remember(&mut recent, &format!("x{i}"), t + WINDOW);
        }
        assert!(recent.len() <= KEEP);
    }
}
