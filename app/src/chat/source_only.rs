//! V108-N12 Shared Chat: a message that stays in this channel. IRC can't say "this channel
//! only", so these go out over Helix (`for_source_only`), which needs the account's
//! `user:write:chat`. The line comes back through the reader like any other.

use cyex_chat::irc::write::MAX_MESSAGE_CHARS;
use cyex_chat::model::ConnStatus;
use tauri::{AppHandle, Manager};

use super::{Chat, Identity, now_ms};

const SCOPE: &str = "user:write:chat";

/// The room's account, channel id and status for a view.
fn target(app: &AppHandle, view: u32) -> Result<(String, String, String), String> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    let key = inner.views.get(&view).map(|v| v.0.clone()).ok_or("This chat is closed.")?;
    let Identity::Account(account) = &key.0 else {
        return Err("Sign in to chat.".into());
    };
    let entry = inner.rooms.get(&key).ok_or("This chat is closed.")?;
    match entry.room.status {
        ConnStatus::Joined => {}
        ConnStatus::AuthFailed => return Err("Twitch didn’t accept this account. Sign in again in Settings.".into()),
        _ => return Err("Not connected yet.".into()),
    }
    let now = now_ms();
    if let Some(until) = entry.room.state.wait_until.filter(|u| *u > now) {
        return Err(format!("You can talk again in {}s.", (until - now + 999) / 1000));
    }
    let channel_id = entry.room.state.channel_id.clone().ok_or("Not connected yet.")?;
    Ok((account.clone(), channel_id, key.1))
}

/// Whether this view's account may send to its own channel only.
pub fn available(app: &AppHandle, view: u32) -> bool {
    let chat = app.state::<Chat>();
    let account = match chat.lock().views.get(&view).map(|v| v.0.0.clone()) {
        Some(Identity::Account(id)) => id,
        _ => return false,
    };
    crate::auth::helix_scoped(app, &account, SCOPE).is_some()
}

/// Sends `text` in this channel only.
pub async fn send(app: &AppHandle, view: u32, text: &str) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let chars = text.chars().count();
    if chars > MAX_MESSAGE_CHARS {
        return Err(format!("Too long: {chars}/{MAX_MESSAGE_CHARS}."));
    }
    let (account, channel_id, login) = target(app, view)?;
    super::cheer::plain_ok(app, &login, text)?;
    let Some((token, client_id)) = crate::auth::helix_scoped(app, &account, SCOPE) else {
        return Err("Sign in again to send to this channel only.".into());
    };
    let http = app.state::<crate::auth::Auth>().http.clone();
    // F20 Helix cuts an unknown command word off and sends the rest.
    let out = super::typed::escape(text.to_owned());
    cyex_twitch::helix::send_message_with(&http, crate::auth::helix_auth(&token, &client_id), &channel_id, &account, &out, true)
        .await
        .map(drop)
        .map_err(|e| match e {
            cyex_twitch::Error::Refused(why) => why,
            e => e.to_string(),
        })
}
