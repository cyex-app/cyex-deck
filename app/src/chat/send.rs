//! Sending (1.8, 1.9): checks before a message leaves, so problems show right away
//! instead of as a Twitch notice. Sends go out on the account's writer connection (V105-06).
//! C21 dodges Twitch's 30 s duplicate check. V111-30 Every written line is answered within
//! `CONFIRM_MS` (USERSTATE or the relayed line) or gets a "Not sent" line with Copy and Retry.

use cyex_chat::irc::{Window, write};
use cyex_chat::model::ConnStatus;
use cyex_chat::{CONFIRM_MS, PendingSend};
use tauri::{AppHandle, Manager};
use tokio::time::{Duration, Instant};

use super::{Chat, Identity, RoomKey, now_ms, pace, typed};

/// V106-08 O4 A channel whose lines showed up this recently is still in the shared chat.
const SHARED_SEEN_MS: i64 = 10 * 60 * 1000;

/// A send the writer dropped (between sessions, logging in, a write error).
const NOT_SENT: &str = "Couldn’t send. Try again.";

/// Sends in a room, shown or not (timers and automations send to rooms no view shows).
/// Done once the line is written, or an error when it couldn't be.
pub async fn send_to(app: &AppHandle, key: &RoomKey, text: &str, reply_to: Option<String>) -> Result<(), String> {
    send_to_until(app, key, text, reply_to, None).await
}

/// [`send_to`] where `cancel` is asked once the pacer's wait is over: true drops the line.
pub async fn send_to_until(
    app: &AppHandle,
    key: &RoomKey,
    text: &str,
    reply_to: Option<String>,
    cancel: Option<Box<dyn Fn() -> bool + Send>>,
) -> Result<(), String> {
    let Some((sent, nonce)) = queue(app, key, text, reply_to, cancel)? else {
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

/// V111-30 The chat box's send: done once the line is checked and queued, so the box clears and
/// the next one can be typed while this one waits its turn. What happens after (the link
/// dropped, Twitch refused it or never answered) shows as a "Not sent" line with Retry.
pub fn send_queued(app: &AppHandle, key: &RoomKey, text: &str, reply_to: Option<String>) -> Result<(), String> {
    let Some((sent, nonce)) = queue(app, key, text, reply_to, None)? else {
        return Ok(());
    };
    let (app, key) = (app.clone(), key.clone());
    tauri::async_runtime::spawn(async move {
        if sent.await {
            return;
        }
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        if let Some(entry) = inner.rooms.get_mut(&key)
            && let Some(ev) = entry.room.not_written(&nonce, now_ms())
        {
            super::queue(entry, [ev], &chat.wake);
        }
    });
    Ok(())
}

/// V111-30 Retry on a "Not sent" line in `view`: the text as it was asked to be sent.
pub fn retry(app: &AppHandle, view: u32, text: &str, reply_to: Option<String>) -> Result<(), String> {
    let key = app.state::<Chat>().lock().views.get(&view).map(|v| v.0.clone()).ok_or("This chat is closed.")?;
    send_queued(app, &key, text, reply_to)
}

/// V111-30 After `CONFIRM_MS` the room's written sends without an answer say so.
fn confirm_later(app: AppHandle, key: RoomKey) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(CONFIRM_MS as u64 + 50)).await;
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        if let Some(entry) = inner.rooms.get_mut(&key) {
            let events = entry.room.unanswered(now_ms());
            if !events.is_empty() {
                super::queue(entry, events, &chat.wake);
            }
        }
    });
}

/// Checks and queues the line; `None` when there was nothing to send.
fn queue(
    app: &AppHandle,
    key: &RoomKey,
    text: &str,
    reply_to: Option<String>,
    cancel: Option<Box<dyn Fn() -> bool + Send>>,
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
        ConnStatus::Banned => return Err("You’re banned here.".into()),
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
    let delay_ms = slot.saturating_duration_since(Instant::now()).as_millis() as i64;
    // C21, V111-30 A repeat changes invisibly, unlike the last one queued and the last one
    // Twitch took; the wording must fit the limit once sent.
    let at_ms = now + delay_ms;
    let taken = entry.room.last_taken(at_ms - pace::DUPLICATE_MS);
    let fits = |w: &str| {
        let n = outgoing(w, action).chars().count();
        if n <= max { Ok(()) } else { Err(n) }
    };
    let picked = pace::pick(entry.last_sent.as_ref(), taken, at_ms, |v| wording(body, v), fits);
    let (_, sent_as) = picked.map_err(|e| match e {
        pace::NoWording::TooLong { chars } => format!("Too long: {chars}/{max}."),
        pace::NoWording::Repeat => "Same message as before. Change it a little.".to_owned(),
    })?;

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
    entry.last_sent = Some(pace::LastSent {
        sent_as: sent_as.clone(),
        at_ms,
    });
    let nonce = nonce();
    entry.room.sending(PendingSend {
        text: body.to_owned(),
        action,
        reply,
        at_ms: now,
        nonce: Some(nonce.clone()),
        typed: text.to_owned(),
        sent_as: sent_as.clone(),
        written_ms: None,
    });
    if let Identity::Account(account) = &key.0 {
        // V106-08 O4 In a shared chat, 7TV hears about every channel of it.
        let shared = entry.room.shared_rooms(now - SHARED_SEEN_MS);
        crate::emotes::presence::touch(app, account, &key.1, crate::emotes::presence::Why::Sent, &shared);
    }
    let line = write::privmsg(
        &key.1,
        &write::Outgoing {
            text: outgoing(&sent_as, action),
            reply_to,
            nonce: Some(nonce.clone()),
            action,
        },
    );
    let (app, room, id) = (app.clone(), key, nonce.clone());
    let sent = async move {
        if slot > Instant::now() {
            tokio::time::sleep_until(slot).await;
        }
        if cancel.is_some_and(|c| c()) {
            return false;
        }
        // The chat was closed while the line waited. V111-30 Twitch's answer is due from here,
        // marked before the write so an answer racing the write's ack still finds it.
        match app.state::<Chat>().lock().rooms.get_mut(&room) {
            Some(e) => e.room.wrote(&id, now_ms()),
            None => return false,
        }
        // V111-30 Test builds with the chat injector send to it, never to Twitch.
        #[cfg(feature = "mocks")]
        let ok = match super::chat_mock::trap_send(&app, &line).await {
            Some(ok) => ok,
            None => client.send(line).await,
        };
        #[cfg(not(feature = "mocks"))]
        let ok = client.send(line).await;
        if ok {
            confirm_later(app, room);
        }
        ok
    };
    Ok(Some((sent, nonce)))
}

/// The text as written to Twitch: F20 ".gg" as text, not as Twitch's command.
fn outgoing(sent_as: &str, action: bool) -> String {
    if action { sent_as.to_owned() } else { typed::escape(sent_as.to_owned()) }
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
