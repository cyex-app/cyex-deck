//! IRC sessions: two clients per account (reader and writer, V105-06), one for anonymous
//! reading, and the tasks that route their lines to rooms.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use cyex_chat::build::Me;
use cyex_chat::irc::{self, Client, Credentials, Event, Line, LinkStatus, Window};
use cyex_chat::model::{ConnStatus, Flag, NoticeKind, RoomEvent};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use super::creds::{AccountCreds, AnonCreds, WriterCreds};
use super::unread::{self, Unread};
use super::writer::Writer;
use super::{Chat, Identity, Inner, RoomKey, Session, account_of, mention, now_ms, queue};

/// The session for `identity`, started if needed.
pub fn ensure<'a>(app: &AppHandle, inner: &'a mut Inner, identity: &Identity) -> &'a Session {
    inner.sessions.entry(identity.clone()).or_insert_with(|| {
        let creds: Arc<dyn Credentials> = match identity {
            Identity::Anon => Arc::new(AnonCreds(app.clone())),
            Identity::Account(id) => Arc::new(AccountCreds::new(app.clone(), id.clone())),
        };
        let (tx, rx) = mpsc::unbounded_channel();
        let (client, task) = Client::new(creds, irc::tls(), tx);
        tauri::async_runtime::spawn(task);
        tauri::async_runtime::spawn(pump(app.clone(), identity.clone(), rx));
        // V105-06 Sends go out on a second connection that joins nothing.
        let writer = match identity {
            Identity::Anon => None,
            Identity::Account(id) => {
                let (wtx, wrx) = mpsc::unbounded_channel();
                let creds = Arc::new(WriterCreds(AccountCreds::new(app.clone(), id.clone())));
                let (wclient, wtask) = Client::new(creds, irc::tls(), wtx);
                tauri::async_runtime::spawn(wtask);
                let writer = Arc::new(Writer::new(wclient));
                tauri::async_runtime::spawn(pump_writer(app.clone(), identity.clone(), writer.clone(), wrx));
                Some(writer)
            }
        };
        Session {
            client,
            writer,
            sends: Window::sends(),
        }
    })
}

/// Sessions Twitch refused log in again, e.g. after the account signed in anew.
pub fn retry_refused(app: &AppHandle) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let inner = chat.lock();
    for (identity, session) in &inner.sessions {
        let refused = inner
            .rooms
            .iter()
            .any(|((id, _), e)| id == identity && e.room.status == ConnStatus::AuthFailed);
        if refused {
            session.client.reconnect();
        }
        // V105-06 The writer can be refused alone, with every chat still joined.
        if let Some(w) = &session.writer {
            if refused {
                w.client.reconnect();
            } else {
                w.retry();
            }
        }
    }
}

async fn pump(app: AppHandle, identity: Identity, mut rx: mpsc::UnboundedReceiver<Event>) {
    while let Some(event) = rx.recv().await {
        match event {
            Event::Status(s) => link_status(&app, &identity, s),
            Event::Line(raw) => line(&app, &identity, &raw),
            Event::Joining(channels) => joining(&app, &identity, channels),
        }
    }
}

/// V105-06 The writer's lines (USERSTATE and NOTICE after a send) go to the rooms like the
/// reader's; its link only decides whether sending is possible.
async fn pump_writer(app: AppHandle, identity: Identity, writer: Arc<Writer>, mut rx: mpsc::UnboundedReceiver<Event>) {
    while let Some(event) = rx.recv().await {
        match event {
            Event::Status(s) => writer.set(s),
            Event::Line(raw) => {
                // An untargeted NOTICE on the writer says nothing about our chats.
                if !Line::parse(&raw).is_some_and(|l| for_every_room(&l)) {
                    line(&app, &identity, &raw)
                }
            }
            Event::Joining(_) => {}
        }
    }
}

/// V105-05 How long Twitch gets to confirm a JOIN. It answers within a second; over the
/// account's chat limit it may stay silent.
const JOIN_ANSWER: std::time::Duration = std::time::Duration::from_secs(20);

/// V105-05 Our JOINs went out: an account's chats Twitch hasn't confirmed by then count
/// as refused. Anonymous chats have no limit.
fn joining(app: &AppHandle, identity: &Identity, channels: Vec<String>) {
    if matches!(identity, Identity::Anon) {
        return;
    }
    let stamp = now_ms();
    {
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        for c in &channels {
            if let Some(e) = inner.rooms.get_mut(&(identity.clone(), c.clone())) {
                e.join_sent = Some(stamp);
            }
        }
    }
    let (app, identity) = (app.clone(), identity.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(JOIN_ANSWER).await;
        unanswered(&app, &identity, &channels, stamp);
    });
}

/// The chats of `channels` still waiting on the JOIN sent at `stamp` become refused.
fn unanswered(app: &AppHandle, identity: &Identity, channels: &[String], stamp: i64) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let now = now_ms();
    let mut any = false;
    for c in channels {
        let Some(e) = inner.rooms.get_mut(&(identity.clone(), c.clone())) else { continue };
        // Sent again since (a reconnect): that JOIN has its own wait.
        if e.join_sent != Some(stamp) {
            continue;
        }
        let events = e.room.join_unanswered(now);
        if events.is_empty() {
            continue;
        }
        eprintln!("chat: no answer to JOIN #{c} in 20 s, counted as refused");
        queue(e, events, &chat.wake);
        any = true;
    }
    drop(inner);
    if any {
        crate::connections::notify(app);
    }
}

fn link_status(app: &AppHandle, identity: &Identity, link: LinkStatus) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let now = now_ms();
    let mut moved = false;
    for ((id, _), entry) in inner.rooms.iter_mut() {
        if id != identity {
            continue;
        }
        // V105-05 A JOIN in flight died with the link: the 20 s wait isn't Twitch's answer.
        if matches!(link, LinkStatus::Reconnecting | LinkStatus::AuthFailed) {
            entry.join_sent = None;
        }
        let status = match link {
            // A fresh connect alone doesn't clear the warning; Twitch's JOIN echo does (V105-05).
            LinkStatus::Connecting if entry.room.status == ConnStatus::Refused => continue,
            // Rooms count as joined once Twitch confirms the JOIN.
            LinkStatus::Connected => continue,
            LinkStatus::Connecting if entry.room.status == ConnStatus::Joined => ConnStatus::Reconnecting,
            LinkStatus::Connecting => continue,
            LinkStatus::Reconnecting => ConnStatus::Reconnecting,
            LinkStatus::AuthFailed => ConnStatus::AuthFailed,
        };
        if status == ConnStatus::Reconnecting {
            entry.room.lost(now);
        }
        moved |= entry.room.status == ConnStatus::Refused;
        let events = entry.room.set_status(status, now);
        queue(entry, events, &chat.wake);
    }
    drop(inner);
    // V105-05 A refused chat that lost its link isn't refused anymore: the counter follows.
    if moved {
        crate::connections::notify(app);
    }
}

/// Our own JOIN: the account's login, or the anonymous `justinfan…` nick. With C37 on,
/// other people's come in too.
fn own(me: Option<&Me>, nick: &str) -> bool {
    match me {
        Some(m) => m.login.eq_ignore_ascii_case(nick),
        None => nick.starts_with("justinfan"),
    }
}

/// V105-05 A NOTICE aimed at no single chat (`*`, `#jtv`). Chatterino shows these in every
/// channel (IrcMessageHandler::handleNoticeMessage); here they go to every chat of the account.
fn for_every_room(line: &Line<'_>) -> bool {
    line.command == "NOTICE" && matches!(line.channel(), None | Some("jtv"))
}

fn line(app: &AppHandle, identity: &Identity, raw: &str) {
    let Some(line) = Line::parse(raw) else { return };
    if for_every_room(&line) {
        // Anonymous chats are never refused; their untargeted notices stay dropped.
        if matches!(identity, Identity::Account(_)) {
            eprintln!("chat: notice for every chat: {raw}");
            let logins: Vec<String> = app
                .state::<Chat>()
                .lock()
                .rooms
                .keys()
                .filter(|(i, _)| i == identity)
                .map(|(_, l)| l.clone())
                .collect();
            for login in &logins {
                room_line(app, identity, login, &line, raw);
            }
        }
        return;
    }
    let Some(channel) = line.channel() else { return };
    room_line(app, identity, channel, &line, raw);
}

fn room_line(app: &AppHandle, identity: &Identity, channel: &str, line: &Line<'_>, raw: &str) {
    let emotes = crate::emotes::room(app, account_of(identity), channel);
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let key = (identity.clone(), channel.to_owned());
    let Some(entry) = inner.rooms.get_mut(&key) else { return };
    // V105-05 The counter changes when a chat turns refused or gets in after all.
    let was_refused = entry.room.status == ConnStatus::Refused;
    // Twitch's answer to a JOIN past the limit isn't documented: keep what it says before
    // a JOIN is confirmed. NOTICEs carry no tokens.
    if line.command == "NOTICE" && entry.room.status != ConnStatus::Joined && matches!(identity, Identity::Account(_)) {
        eprintln!("chat: before joining #{channel}: {raw}");
    }

    if line.command == "JOIN" && own(entry.room.me(), line.nick()) {
        entry.join_sent = None;
        let events = entry.room.set_status(ConnStatus::Joined, now_ms());
        queue(entry, events, &chat.wake);
        let gap = entry.room.take_gap(now_ms());
        drop(inner);
        if was_refused {
            crate::connections::notify(app);
        }
        if let Some(span) = gap {
            fill_gap(app.clone(), key, span);
        }
        return;
    }
    // C37 Lines asked for before the setting went off, until the next connect.
    if matches!(line.command, "JOIN" | "PART") && !chat.joins.load(Ordering::Relaxed) {
        return;
    }
    let role_before = entry.room.role();
    let events = entry.room.handle_line(line, now_ms(), &emotes);
    let refused = entry.room.status == ConnStatus::Refused;
    let role = entry.room.role();
    // U9 Only rooms no view shows collect news.
    let news = if entry.views.is_empty() { unread::level(&events) } else { Unread::Seen };
    let lit = news > entry.unread;
    if lit {
        entry.unread = news;
    }
    let own: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RoomEvent::Message { message } if message.has(Flag::Own) && !message.has(Flag::History) => {
                Some(message.clone())
            }
            _ => None,
        })
        .collect();
    let outcomes = entry.room.take_outcomes();
    let live: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            RoomEvent::Message { message } if !message.has(Flag::History) => Some(message.clone()),
            _ => None,
        })
        .collect();
    super::record::events(app, &key, &events);
    let moderated = events
        .iter()
        .any(|e| matches!(e, RoomEvent::Delete { .. } | RoomEvent::ClearUser { .. }))
        .then(|| events.clone());
    queue(entry, events, &chat.wake);
    drop(inner);

    if refused != was_refused {
        crate::connections::notify(app);
    }
    // V102-21 A role change here: mod feeds (1.46) and every window.
    if role != role_before && let Identity::Account(_) = identity {
        super::roles::changed(app);
    }
    let muted = super::rules::current(app).muted(channel);
    for (m, outcome) in outcomes {
        if outcome.pings() && !muted {
            mention::notify(app, channel, &m, &outcome, false);
        }
        if !outcome.routes.is_empty() {
            super::feeds::route(app, channel, &m, &outcome.routes);
        }
    }
    if let Some(events) = moderated {
        super::feeds::moderate(app, channel, &events);
    }
    if !live.is_empty() {
        super::translate::auto(app, &key, &live);
        for m in &live {
            super::run::automate(app, &key, m);
        }
    }
    for m in own {
        crate::emotes::record_usage(app, &m.body);
    }
    if lit {
        unread::emit(app, &key, news);
    }
}

/// C24 Fetches what was said while the connection was gone and puts it in by time.
fn fill_gap(app: AppHandle, key: RoomKey, (after, before, limit): (i64, i64, u32)) {
    let history = app.state::<crate::settings::Settings>().read::<cyex_settings::schema::Chat>(cyex_settings::Partition::Chat).history;
    if !history {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let http = app.state::<crate::auth::Auth>().http.clone();
        let lines = match cyex_twitch::recent::recent_between(&http, &key.1, after, before, limit).await {
            Ok(lines) => lines,
            Err(e) => {
                eprintln!("chat: couldn't fill the gap in {}: {}", key.1, e.describe("recent-messages"));
                say(&app, &key, "Couldn’t load what was missed while disconnected");
                return;
            }
        };
        let emotes = crate::emotes::room(&app, account_of(&key.0), &key.1);
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        let Some(entry) = inner.rooms.get_mut(&key) else { return };
        let messages = entry.room.fill_gap(&lines, now_ms(), &emotes);
        if !messages.is_empty() {
            queue(entry, [RoomEvent::Gap { messages }], &chat.wake);
        }
    });
}

/// C35 How far back history from logs.cyex.app may go.
const LOGS_HISTORY_SECS: i64 = 24 * 3600;

/// C1 Fetches up to `limit` recent messages in the background and adds them above the
/// live ones. C35 Where the history service has none, logs.cyex.app stands in.
pub fn load_history(app: AppHandle, key: RoomKey, limit: u32) {
    tauri::async_runtime::spawn(async move {
        let http = app.state::<crate::auth::Auth>().http.clone();
        let lane = super::shown::lane(&app, &key.1);
        // V107-M1 A chat joined in the background takes a little; shown, it gets the rest.
        let background = app.state::<Chat>().lock().rooms.get(&key).is_some_and(|e| e.room.background());
        let asked = if background { limit.min(cyex_chat::BACKGROUND_HISTORY) } else { limit };
        let lines = fetch_history(&app, &http, &key, asked, lane).await;
        let emotes = crate::emotes::room(&app, account_of(&key.0), &key.1);
        let refill = {
            let chat = app.state::<Chat>();
            let mut inner = chat.lock();
            let Some(entry) = inner.rooms.get_mut(&key) else { return };
            if let Some(lines) = lines {
                let messages = entry.room.history(&lines, now_ms(), &emotes);
                if !messages.is_empty() {
                    queue(entry, [RoomEvent::History { messages }], &chat.wake);
                }
            }
            // V107-M1 Only now may a refill run: it starts above these lines.
            entry.loaded = true;
            super::refill::first_history_in(&mut entry.room, asked < limit, !entry.views.is_empty())
        };
        if refill {
            super::refill::start(&app, key);
        }
    });
}

/// C1 Up to `limit` history lines; logs.cyex.app stands in where the service has none
/// (C35). `None`: neither had any, said in chat.
async fn fetch_history(
    app: &AppHandle,
    http: &cyex_twitch::Http,
    key: &RoomKey,
    limit: u32,
    lane: cyex_twitch::pace::Lane,
) -> Option<Vec<String>> {
    let other = match cyex_twitch::recent::recent_messages(http, &key.1, limit, lane).await {
        Ok(lines) if !lines.is_empty() => return Some(lines),
        other => other,
    };
    let now = now_ms() / 1000;
    let from = cyex_twitch::clock::rfc3339(now - LOGS_HISTORY_SECS);
    let to = cyex_twitch::clock::rfc3339(now);
    match cyex_twitch::logs::channel_lines(http, &key.1, limit, &from, &to).await {
        Ok(lines) if !lines.is_empty() => {
            say(app, key, "Chat history from logs.cyex.app");
            Some(lines)
        }
        _ => {
            if let Err(e) = other {
                eprintln!("chat: no history for {}: {}", key.1, e.describe("recent-messages"));
                say(app, key, &cyex_twitch::recent::unavailable(&e));
            }
            None
        }
    }
}

/// V101-32 A line of ours in a room that's still open.
fn say(app: &AppHandle, key: &RoomKey, text: &str) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let Some(entry) = inner.rooms.get_mut(key) else { return };
    let ev = entry.room.system(NoticeKind::Info, text, now_ms());
    queue(entry, [ev], &chat.wake);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_own_join_counts_as_joined() {
        let me = Me::new("1", "me", "Me");
        assert!(own(Some(&me), "me"));
        assert!(!own(Some(&me), "ana"), "C37: someone else's JOIN");
        assert!(own(None, "justinfan4821"));
        assert!(!own(None, "ana"));
    }

    fn every(raw: &str) -> bool {
        for_every_room(&Line::parse(raw).unwrap())
    }

    #[test]
    fn untargeted_notices_go_to_every_chat() {
        assert!(every(":tmi.twitch.tv NOTICE * :x"));
        assert!(every(":tmi.twitch.tv NOTICE #jtv :x"));
        assert!(!every(":tmi.twitch.tv NOTICE #chan :x"));
        assert!(!every(":a!a@a PRIVMSG #chan :x"));
    }
}
