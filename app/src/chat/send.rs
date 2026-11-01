//! Sending (1.8, 1.9): checks before a message leaves, so problems show right away
//! instead of as a Twitch notice. Sends go out on the account's writer connection (V105-06).
//! C21 dodges Twitch's 30 s duplicate check.

use cyex_chat::PendingSend;
use cyex_chat::irc::{Window, write};
use cyex_chat::model::ConnStatus;
use tauri::{AppHandle, Manager};
use tokio::time::Instant;

use super::{Chat, Identity, RoomKey, now_ms, pace, typed};

/// V106-08 O4 A channel whose lines showed up this recently is still in the shared chat.
const SHARED_SEEN_MS: i64 = 10 * 60 * 1000;

/// A send the writer dropped (between sessions, logging in, a write error).
const NOT_SENT: &str = "Couldn’t send. Try again.";

/// Sends in a room, shown or not (timers and automations send to rooms no view shows).
/// Done once the line is written, or an error when it couldn't be.
pub async fn send_to(app: &AppHandle, key: &RoomKey, text: &str, reply_to: Option<String>) -> Result<(), String> {
    let Some((sent, nonce)) = queue(app, key, text, reply_to)? else {
        return Ok(());
    };
    if sent.await {
        return Ok(());
    }
    // Twitch won't answer it: its echo can't take the next send's place.
    let chat = app.state::<Chat>();
    if let Some(entry) = chat.lock().rooms.get_mut(key) {
        entry.room.unsend(&nonce);
    }
    Err(NOT_SENT.into())
}

/// Checks and queues the line; `None` when there was nothing to send.
fn queue(
    app: &AppHandle,
    key: &RoomKey,
    text: &str,
    reply_to: Option<String>,
) -> Result<Option<(impl Future<Output = bool> + use<>, String)>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    // V107-N1c A bare "/me" isn’t an unknown command; it just has no text.
    if text.eq_ignore_ascii_case("/me") || text.eq_ignore_ascii_case(".me") {
        return Err("Write something after /me.".into());
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
    let (chars, max) = (body.chars().count(), max_chars(action));
    if chars > max {
        return Err(format!("Too long: {chars}/{max}."));
    }
    // V107-Bn A cheer word goes out as text only where Bits are off.
    super::cheer::plain_ok(app, &key.1, text)?;

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
        ConnStatus::AuthFailed => return Err("Twitch didn’t accept this account. Sign in again in Settings.".into()),
        _ => return Err("Not connected yet.".into()),
    }
    if let Some(until) = entry.room.state.wait_until.filter(|u| *u > now) {
        return Err(format!("You can talk again in {}s.", (until - now + 999) / 1000));
    }
    let fast = entry.room.state.own.as_ref().is_some_and(|o| o.fast_lane());
    let reply = reply_to.as_deref().and_then(|id| entry.room.reply_to(id));
    // V108-C15 A normal chatter's sends wait their turn (about one a second per channel).
    let mut pacer = entry.pacer;
    let slot = if fast {
        Instant::now()
    } else {
        pacer.take(Instant::now()).ok_or("Slow down: too many waiting.")?
    };

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
    entry.pacer = pacer;
    let delay_ms = slot.saturating_duration_since(Instant::now()).as_millis() as i64;
    let variant = pace::LastSent::next_variant(entry.last_sent.as_ref(), body, now + delay_ms);
    let out = wording(body, variant);
    entry.last_sent = Some(pace::LastSent {
        body: body.to_owned(),
        variant,
        at_ms: now + delay_ms,
    });
    let nonce = nonce();
    entry.room.sending(PendingSend {
        text: body.to_owned(),
        action,
        reply,
        at_ms: now,
        nonce: Some(nonce.clone()),
    });
    if let Identity::Account(account) = &key.0 {
        // V106-08 O4 In a shared chat, 7TV hears about every channel of it.
        let shared = entry.room.shared_rooms(now - SHARED_SEEN_MS);
        crate::emotes::presence::touch(app, account, &key.1, crate::emotes::presence::Why::Sent, &shared);
    }
    // F20 ".gg" as text, not as Twitch's command.
    let out = if action { out } else { typed::escape(out) };
    let line = write::privmsg(
        &key.1,
        &write::Outgoing {
            text: out,
            reply_to,
            nonce: Some(nonce.clone()),
            action,
        },
    );
    let (app, room) = (app.clone(), key);
    let sent = async move {
        if slot > Instant::now() {
            tokio::time::sleep_until(slot).await;
            // The chat was closed while the line waited.
            if !app.state::<Chat>().lock().rooms.contains_key(&room) {
                return false;
            }
        }
        client.send(line).await
    };
    Ok(Some((sent, nonce)))
}

/// A `client-nonce` unique to this send, so its USERSTATE finds it: 32 random lowercase hex, as
/// twitch.tv sends. Our own "cyex…" one read as abnormal to chat tools (user, 2026/11/01).
fn nonce() -> String {
    cyex_twitch::random::hex(16)
}

/// V107-N1a Twitch counts the ACTION wrapper (9 chars) of a /me line toward the 500; a longer one
/// is dropped without a word.
fn max_chars(action: bool) -> usize {
    write::MAX_MESSAGE_CHARS - if action { 9 } else { 0 }
}

/// Chatterino's trick: double the first space, or add an invisible character. Runs before
/// [`typed::escape`], so the first space of ". hi" is text and doubling it works.
fn dodge_duplicate(text: &str) -> String {
    match text.find(' ') {
        Some(i) => format!("{} {}", &text[..i], &text[i..]),
        None => format!("{text} {}", typed::INVISIBLE),
    }
}

/// V108-C15 The text as it goes out: the user's own, or a repeat's other wordings.
fn wording(text: &str, variant: u8) -> String {
    match variant {
        0 => text.to_owned(),
        1 => dodge_duplicate(text),
        _ => format!("{}{}", dodge_duplicate(text), typed::INVISIBLE),
    }
}

#[cfg(test)]
mod tests {
    use super::{dodge_duplicate, max_chars, nonce, typed, wording};
    use cyex_chat::model::Client;

    #[test]
    fn an_action_leaves_room_for_its_wrapper() {
        assert_eq!(max_chars(false), 500);
        assert_eq!(max_chars(true), 491);
    }

    #[test]
    fn nonces_differ_and_look_like_the_website() {
        let (a, b) = (nonce(), nonce());
        assert_ne!(a, b);
        assert_eq!(Client::from_nonce(&a), Some(Client::Web));
    }

    #[test]
    fn duplicates_change_invisibly() {
        assert_eq!(dodge_duplicate("hi there"), "hi  there");
        assert_eq!(dodge_duplicate("hi"), "hi \u{34F}");
    }

    #[test]
    fn repeat_wordings_all_differ() {
        let w: Vec<_> = (0..3).map(|v| wording("A", v)).collect();
        assert_eq!(w[0], "A");
        assert!(w[0] != w[1] && w[1] != w[2] && w[0] != w[2]);
        let w: Vec<_> = (0..3).map(|v| wording("a b", v)).collect();
        assert!(w[0] != w[1] && w[1] != w[2] && w[0] != w[2]);
    }

    #[test]
    fn a_dot_line_sent_twice_stays_text() {
        assert_eq!(typed::escape(dodge_duplicate(". hi")), "\u{34F}.  hi");
        assert_eq!(typed::escape(dodge_duplicate(".gg")), "\u{34F}.gg \u{34F}");
    }
}
