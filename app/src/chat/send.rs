//! Sending (1.8, 1.9): checks before a message leaves, so problems show right away
//! instead of as a Twitch notice. Sends go out on the account's writer connection (V105-06).
//! C21 dodges Twitch's 30 s duplicate check.

use cyex_chat::PendingSend;
use cyex_chat::irc::{Window, write};
use cyex_chat::model::ConnStatus;
use tauri::{AppHandle, Manager};
use tokio::time::Instant;

use super::{Chat, Identity, RoomKey, now_ms, typed};

/// Twitch rejects the same text twice within this window.
const DUPLICATE_MS: i64 = 30_000;

/// Sends in a room, shown or not (timers and automations send to rooms no view shows).
pub fn send_to(app: &AppHandle, key: &RoomKey, text: &str, reply_to: Option<String>) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let (body, action) = match text.strip_prefix("/me ").or_else(|| text.strip_prefix(".me ")) {
        Some(rest) => (rest.trim(), true),
        // F14 Commands don't go out as text: any "/" one, and a dot before the app's or
        // Twitch's own ones. ".gg" does.
        None if super::typed::command(text, super::run::reserved).is_some() => {
            let word = text.split_whitespace().next().unwrap_or(text);
            return Err(format!("Unknown command {word}."));
        }
        None => (text, false),
    };
    let chars = text.chars().count();
    if chars > write::MAX_MESSAGE_CHARS {
        return Err(format!("Too long: {chars}/{}.", write::MAX_MESSAGE_CHARS));
    }

    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let key = key.clone();
    if key.0 == Identity::Anon {
        return Err("Sign in to chat.".into());
    }
    let now = now_ms();
    let entry = inner.rooms.get(&key).ok_or("This chat is closed.")?;
    match entry.room.status {
        ConnStatus::Joined => {}
        ConnStatus::AuthFailed => return Err("Twitch didn't accept this account. Sign in again in Settings.".into()),
        _ => return Err("Not connected yet.".into()),
    }
    if let Some(until) = entry.room.state.wait_until.filter(|u| *u > now) {
        return Err(format!("You can talk again in {}s.", (until - now + 999) / 1000));
    }
    let fast = entry.room.state.own.as_ref().is_some_and(|o| o.fast_lane());
    let reply = reply_to.as_deref().and_then(|id| entry.room.reply_to(id));

    let session = inner.sessions.get_mut(&key.0).ok_or("Not connected yet.")?;
    // V105-06 Sends go out on the account's writer.
    let Some(writer) = session.writer.as_ref() else {
        return Err("Sign in to chat.".into());
    };
    if let Some(why) = writer.block() {
        return Err(why.into());
    }
    let client = writer.client.clone();
    let limit = if fast { Window::SEND_LIMIT_FAST } else { Window::SEND_LIMIT };
    if !session.sends.try_take(limit, Instant::now()) {
        let wait = session.sends.wait(limit, Instant::now()).unwrap_or_default();
        return Err(format!("Slow down: wait {}s.", wait.as_secs() + 1));
    }

    let entry = inner.rooms.get_mut(&key).ok_or("This chat is closed.")?;
    let mut out = body.to_owned();
    if let Some((last, at)) = &entry.last_sent
        && *last == out
        && now - at < DUPLICATE_MS
    {
        out = dodge_duplicate(&out);
    }
    entry.last_sent = Some((out.clone(), now));
    entry.room.sending(PendingSend {
        text: body.to_owned(),
        action,
        reply,
        at_ms: now,
    });
    if let Identity::Account(account) = &key.0 {
        crate::emotes::presence::touch(app, account, &key.1);
    }
    // F20 ".gg" as text, not as Twitch's command.
    let out = if action { out } else { typed::escape(out) };
    client.send(write::privmsg(
        &key.1,
        &write::Outgoing {
            text: out,
            reply_to,
            nonce: None,
            action,
        },
    ));
    Ok(())
}

/// Chatterino's trick: double the first space, or add an invisible character. Runs before
/// [`typed::escape`], so the first space of ". hi" is text and doubling it works.
fn dodge_duplicate(text: &str) -> String {
    match text.find(' ') {
        Some(i) => format!("{} {}", &text[..i], &text[i..]),
        None => format!("{text} {}", typed::INVISIBLE),
    }
}

#[cfg(test)]
mod tests {
    use super::{dodge_duplicate, typed};

    #[test]
    fn duplicates_change_invisibly() {
        assert_eq!(dodge_duplicate("hi there"), "hi  there");
        assert_eq!(dodge_duplicate("hi"), "hi \u{34F}");
    }

    #[test]
    fn a_dot_line_sent_twice_stays_text() {
        assert_eq!(typed::escape(dodge_duplicate(". hi")), "\u{34F}.  hi");
        assert_eq!(typed::escape(dodge_duplicate(".gg")), "\u{34F}.gg \u{34F}");
    }
}
