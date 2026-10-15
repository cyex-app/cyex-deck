//! U6, 2.6, V104-01 Highlights and mentions that asked for it flash the taskbar, show a
//! Windows pop-up, or play the sound (the chime, or a mention's own file; V1051-01 in Rust).

use cyex_chat::model::{ChatMessage, plain};
use cyex_chat::rules::Outcome;
use tauri::AppHandle;

use crate::toasts::{self, Open, Pop};

/// A pop-up keeps this much of the message.
const PREVIEW_CHARS: usize = 120;

/// `global`: from logs.cyex.app's firehose (V104-02); its pop-up opens the `#channel`
/// window (V102-05) instead of a tab.
pub fn notify(app: &AppHandle, channel: &str, m: &ChatMessage, outcome: &Outcome, global: bool) {
    if outcome.flash {
        toasts::flash(app);
    }
    let from = || m.sender.as_ref().map(|s| s.display.clone()).unwrap_or_default();
    if outcome.sound {
        crate::sound::alert(app, outcome.sound_file.as_deref());
    }
    if outcome.notify && !toasts::app_in_front(app) {
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
