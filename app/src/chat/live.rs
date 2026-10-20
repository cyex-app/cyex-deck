//! 1.9 Live pins and redemptions without a message, over Hermes, for every joined room.
//! Rewards that take a message show up in IRC already, so only the others get a line.

use cyex_chat::model::NoticeKind;
use cyex_core::number::grouped;
use cyex_twitch::hermes::{Event, Kind};
use serde_json::Value;
use tauri::{AppHandle, Manager};

const KINDS: [Kind; 2] = [Kind::Pins, Kind::Points];

pub fn init(app: &AppHandle) {
    app.state::<super::Chat>().channels.hook(on_channel);
}

/// V107-R16 A joined channel's id is known: follow its pins and redemptions, until its last
/// room is left.
fn on_channel(app: &AppHandle, e: &super::channels::Event) {
    use super::channels::Event;
    match e {
        Event::Known { id, .. } => {
            for kind in KINDS {
                crate::hermes::add(app, kind, id);
            }
        }
        Event::Gone { id: Some(id), .. } => {
            for kind in KINDS {
                crate::hermes::remove(app, kind, id);
            }
        }
        _ => {}
    }
}

/// The joined channel with this id.
fn login_of(app: &AppHandle, channel_id: &str) -> Option<String> {
    app.state::<super::Chat>().channels.login_of(channel_id)
}

/// V107 We pinned or unpinned in this channel: read its pin now rather than wait for
/// Hermes, so the banner swaps to the new pin at once (Hermes still covers pins made elsewhere).
pub fn pin_changed(app: &AppHandle, channel_id: &str) {
    if let Some(login) = login_of(app, channel_id) {
        refresh(app, login, channel_id.to_owned());
    }
}

/// V107 A message was deleted: if it's the pinned one, read the pin again, since Twitch
/// takes it down with the message. Other deletes cost nothing.
pub fn message_gone(app: &AppHandle, channel_id: &str, msg_id: &str) {
    let pinned = {
        let chat = app.state::<super::Chat>();
        let inner = chat.lock();
        inner.rooms.values().any(|e| e.room.pins(msg_id))
    };
    if pinned {
        pin_changed(app, channel_id);
    }
}

/// V107-P4/P7 Who sent message `msg_id` and its plain text, from any open chat, for the delete notice.
pub fn deleted_of(app: &AppHandle, msg_id: &str) -> (Option<String>, Option<String>) {
    let chat = app.state::<super::Chat>();
    let inner = chat.lock();
    match inner.rooms.values().find_map(|e| e.room.sender_and_text(msg_id)) {
        Some((who, text)) => (who, Some(text)),
        None => (None, None),
    }
}

/// Reads the channel's pin once into every room open on it; the new one replaces the old.
fn refresh(app: &AppHandle, login: String, channel_id: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        super::catalog::refresh_pins(&app, &login, &channel_id).await;
    });
}

pub fn on_event(app: &AppHandle, e: &Event) {
    let Some(login) = login_of(app, &e.topic.channel_id) else { return };
    match e.topic.kind {
        Kind::Pins => refresh(app, login, e.topic.channel_id.clone()),
        Kind::Points => {
            if let Some(text) = redemption_line(&e.payload) {
                super::notice_line(app, &login, NoticeKind::Redemption, &text);
            }
        }
        _ => {}
    }
}

/// "Name redeemed Hydrate · 500", for rewards redeemed without a message.
fn redemption_line(payload: &Value) -> Option<String> {
    if payload.get("type")?.as_str()? != "reward-redeemed" {
        return None;
    }
    let r = payload.pointer("/data/redemption")?;
    let reward = r.get("reward")?;
    let has_text = r
        .get("user_input")
        .and_then(Value::as_str)
        .is_some_and(|t| !t.trim().is_empty());
    if has_text || reward.get("is_user_input_required").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let user = r.get("user")?;
    let name = user
        .get("display_name")
        .and_then(Value::as_str)
        .filter(|n| !n.is_empty())
        .or_else(|| user.get("login").and_then(Value::as_str))?;
    let title = reward.get("title")?.as_str()?;
    Some(match reward.get("cost").and_then(Value::as_u64) {
        Some(cost) if cost > 0 => format!("{name} redeemed {title} · {}", grouped(cost)),
        _ => format!("{name} redeemed {title}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn redeemed(input: Option<&str>, required: bool) -> Value {
        let mut r = json!({"type":"reward-redeemed","data":{"redemption":{
            "user":{"id":"1","login":"kuni","display_name":"Kuni"},
            "reward":{"title":"Louder","cost":3220,"is_user_input_required":required}}}});
        if let Some(t) = input {
            r["data"]["redemption"]["user_input"] = json!(t);
        }
        r
    }

    #[test]
    fn only_rewards_without_a_message_get_a_line() {
        assert_eq!(
            redemption_line(&redeemed(None, false)).as_deref(),
            Some("Kuni redeemed Louder · 3.220")
        );
        assert_eq!(redemption_line(&redeemed(Some("hi"), true)), None);
        assert_eq!(redemption_line(&redeemed(None, true)), None);
        assert_eq!(
            redemption_line(&json!({"type":"community-goal-updated","data":{}})),
            None
        );
    }
}
