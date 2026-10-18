//! U6, 2.6, V104-01 Highlights and mentions that asked for it flash the taskbar, show a
//! Windows pop-up, or play the sound (the chime, or a mention's own file; V1051-01 in Rust).
//! V106-25 One message plays one sound, however many chats show it.

use std::collections::VecDeque;
use std::sync::Mutex;

use cyex_chat::model::{ChatMessage, plain};
use cyex_chat::rules::Outcome;
use tauri::AppHandle;

use crate::toasts::{self, Open, Pop};

/// A pop-up keeps this much of the message.
const PREVIEW_CHARS: usize = 120;
/// V106-25 Message ids that played a sound lately: the same chat open twice (two tabs, two
/// accounts, the global firehose) gets each message more than once.
const RECENT_SOUNDS: usize = 64;
static SOUNDED: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// `global`: from logs.cyex.app's firehose (V104-02); its pop-up opens the `#channel`
/// window (V102-05) instead of a tab. V106-25 No sound from a channel that is only in muted tabs.
/// Each message sounds once.
pub fn notify(app: &AppHandle, channel: &str, m: &ChatMessage, outcome: &Outcome, global: bool) {
    if outcome.flash {
        toasts::flash(app);
    }
    let from = || m.sender.as_ref().map(|s| s.display.clone()).unwrap_or_default();
    // R9 One message gets one sound and one pop-up; flash dedups through `FLASHING`.
    let first = first_sound(&m.id);
    if outcome.sound && !crate::tabs::mute::hushes_login(app, channel) && first {
        crate::sound::alert(app, outcome.sound_file.as_deref());
    }
    if outcome.notify && first && !toasts::app_in_front(app) {
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

/// V106-25 False when this message already played its sound.
fn first_sound(id: &str) -> bool {
    remember(&mut SOUNDED.lock().unwrap_or_else(|p| p.into_inner()), id, RECENT_SOUNDS)
}

/// Adds `id` to the last `keep` ids; false when it was there already. No id: always new.
fn remember(recent: &mut VecDeque<String>, id: &str, keep: usize) -> bool {
    if id.is_empty() {
        return true;
    }
    if recent.iter().any(|r| r == id) {
        return false;
    }
    if recent.len() >= keep {
        recent.pop_front();
    }
    recent.push_back(id.to_owned());
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_seen_twice_sounds_once() {
        let mut recent = VecDeque::new();
        assert!(remember(&mut recent, "m1", 2));
        assert!(!remember(&mut recent, "m1", 2));
        assert!(remember(&mut recent, "m2", 2));
        assert!(!remember(&mut recent, "m1", 2));
    }

    #[test]
    fn old_ids_drop_out_and_empty_ids_always_sound() {
        let mut recent = VecDeque::new();
        assert!(remember(&mut recent, "m1", 2));
        assert!(remember(&mut recent, "m2", 2));
        assert!(remember(&mut recent, "m3", 2));
        assert!(remember(&mut recent, "m1", 2));
        assert!(remember(&mut recent, "", 2));
        assert!(remember(&mut recent, "", 2));
    }
}
