//! U6, 2.6 Highlights that asked for it flash the taskbar or tell the main window to play
//! the sound.

use cyex_chat::model::{ChatMessage, plain};
use cyex_chat::rules::Outcome;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, UserAttentionType};
use ts_rs::TS;

const EVENT: &str = "chat:mention";

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Mention {
    pub channel: String,
    pub from: String,
    pub text: String,
}

pub fn notify(app: &AppHandle, channel: &str, m: &ChatMessage, outcome: &Outcome) {
    if outcome.flash
        && let Some(w) = app.get_webview_window("main")
        && !w.is_focused().unwrap_or(true)
    {
        let _ = w.request_user_attention(Some(UserAttentionType::Informational));
    }
    if outcome.sound {
        let _ = app.emit_to(
            "main",
            EVENT,
            Mention {
                channel: channel.to_owned(),
                from: m.sender.as_ref().map(|s| s.display.clone()).unwrap_or_default(),
                text: plain(&m.body),
            },
        );
    }
}
