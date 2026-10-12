//! U6, 2.6, V104-01 Highlights and mentions that asked for it flash the taskbar, show a
//! Windows pop-up, or tell the main window to play the sound (the chime, or a mention's
//! own file).

use cyex_chat::model::{ChatMessage, plain};
use cyex_chat::rules::Outcome;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use crate::toasts::{self, Open, Pop};

const EVENT: &str = "chat:mention";
/// A pop-up keeps this much of the message.
const PREVIEW_CHARS: usize = 120;

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Mention {
    pub channel: String,
    pub from: String,
    pub text: String,
    /// V104-01 A sound file to play instead of the chime.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sound: Option<String>,
}

/// `global`: from logs.cyex.app's firehose (V104-02); its pop-up opens the `#channel`
/// window (V102-05) instead of a tab.
pub fn notify(app: &AppHandle, channel: &str, m: &ChatMessage, outcome: &Outcome, global: bool) {
    if outcome.flash {
        toasts::flash(app);
    }
    let from = || m.sender.as_ref().map(|s| s.display.clone()).unwrap_or_default();
    if outcome.sound && !crate::streamer::hides(app, |s| s.mute_sounds) {
        let _ = app.emit_to(
            crate::windows::MAIN,
            EVENT,
            Mention {
                channel: channel.to_owned(),
                from: from(),
                text: plain(&m.body),
                sound: outcome.sound_file.clone(),
            },
        );
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
