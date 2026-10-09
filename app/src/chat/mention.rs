//! U6 Mentions: flash the taskbar and tell the main window to play the sound, if the
//! user turned those on.

use cyex_chat::model::{ChatMessage, plain};
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, UserAttentionType};
use ts_rs::TS;

use crate::settings::Settings;

const EVENT: &str = "chat:mention";

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Mention {
    pub channel: String,
    pub from: String,
    pub text: String,
}

pub fn notify(app: &AppHandle, channel: &str, m: &ChatMessage) {
    let s: ChatSettings = app.state::<Settings>().read(Partition::Chat);
    if !s.mention_sound && !s.mention_flash {
        return;
    }
    if s.mention_flash
        && let Some(w) = app.get_webview_window("main")
        && !w.is_focused().unwrap_or(true)
    {
        let _ = w.request_user_attention(Some(UserAttentionType::Informational));
    }
    if s.mention_sound {
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
