//! IRC sessions: two clients per account (reader and writer, V105-06), one for anonymous
//! reading, and the tasks that route their lines to rooms.

use std::sync::Arc;

use cyex_chat::build::Me;
use cyex_chat::irc::{self, Client, Credentials, Event, Line, LinkStatus, Window};
use cyex_chat::model::{ConnStatus, Flag, NoticeKind, RoomEvent};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use super::creds::{AccountCreds, AnonCreds, WriterCreds};
use super::unread::{self, News};
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

/// The chats of `channels` still waiting on the JOIN sent at `stamp` ask once more; the
/// second silence marks them not joined (V109-F80: not the join limit).
fn unanswered(app: &AppHandle, identity: &Identity, channels: &[String], stamp: i64) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let now = now_ms();
    let mut any = false;
    let mut again = Vec::new();
    for c in channels {
        let Some(e) = inner.rooms.get_mut(&(identity.clone(), c.clone())) else { continue };
        // Sent again since (a reconnect): that JOIN has its own wait.
        if e.join_sent != Some(stamp) {
            continue;
        }
        if !e.join_retried && e.room.waiting() {
            e.join_retried = true;
            again.push(c.clone());
            continue;
        }
        let events = e.room.join_unanswered(now);
        if events.is_empty() {
            continue;
        }
        eprintln!("chat: no answer to JOIN #{c} twice, not joined");
        queue(e, events, &chat.wake);
        any = true;
    }
    if let Some(session) = inner.sessions.get(identity) {
        for c in &again {
            eprintln!("chat: no answer to JOIN #{c} in 20 s, asking again");
            session.client.rejoin(c);
        }
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
            entry.join_retried = false;
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

/// V109-F81 Joins a chat that isn't in (banned and now maybe not, unanswered, refused) once
/// more. `only_banned`: just a chat Twitch kept from the account for a ban (the ban may
/// have ended). True when a JOIN went out.
pub fn rejoin(app: &AppHandle, key: &RoomKey, only_banned: bool) -> bool {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let Some(e) = inner.rooms.get_mut(key) else { return false };
    let stuck = match e.room.status {
        ConnStatus::Banned => true,
        ConnStatus::NoAnswer | ConnStatus::Refused | ConnStatus::Connecting => !only_banned,
        _ => false,
    };
    if !stuck {
        return false;
    }
    e.join_retried = false;
    let was_refused = e.room.status == ConnStatus::Refused;
    let events = e.room.set_status(ConnStatus::Connecting, now_ms());
    queue(e, events, &chat.wake);
    if let Some(session) = inner.sessions.get(&key.0) {
        session.client.rejoin(&key.1);
    }
    drop(inner);
    if was_refused {
        crate::connections::notify(app);
    }
    true
}

/// V109-F81 The account's chats of `channel_id` that a ban kept out, joined again.
pub fn rejoin_banned(app: &AppHandle, account: &str, channel_id: &str) {
    let keys: Vec<RoomKey> = app
        .state::<Chat>()
        .lock()
        .rooms
        .iter()
        .filter(|((id, _), e)| {
            matches!(id, Identity::Account(a) if a == account)
                && e.room.status == ConnStatus::Banned
                && e.room.state.channel_id.as_deref() == Some(channel_id)
        })
        .map(|(k, _)| k.clone())
        .collect();
    for k in keys {
        rejoin(app, &k, true);
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
    // V110-M28 The anonymous connection also feeds the banned rooms that read through it.
    if matches!(identity, Identity::Anon) {
        mirror(app, channel, &line, raw);
    }
    room_line(app, identity, channel, &line, raw);
}

/// V110-T05 Test builds only: a raw incoming line from the chat injector, into every open
/// room of the channel by the same path as a line off the wire.
#[cfg(feature = "mocks")]
pub(super) fn inject(app: &AppHandle, channel: &str, raw: &str) {
    let Some(line) = Line::parse(raw) else { return };
    let ids: Vec<Identity> =
        app.state::<Chat>().lock().rooms.keys().filter(|(_, c)| c == channel).map(|(i, _)| i.clone()).collect();
    for id in &ids {
        room_line(app, id, channel, &line, raw);
    }
}

/// V110-M28 What a room's anonymous mirror does after a status change.
#[derive(Debug, PartialEq, Eq)]
enum Mirror {
    Start,
    Stop,
}

/// V110-M28 Whether a room starts or stops reading through the anonymous connection. Starts
/// when Twitch keeps the chat from the account for a ban and the setting is on; stops once
/// the account is in (or the setting is off). Trying to get back in (`Connecting`,
/// `Reconnecting`) keeps it, so a failed rejoin leaves no gap.
fn mirror_step(account: bool, status: ConnStatus, mirrored: bool, read: bool) -> Option<Mirror> {
    if !account {
        return None;
    }
    match (mirrored, status) {
        (false, ConnStatus::Banned) if read => Some(Mirror::Start),
        (true, _) if !read => Some(Mirror::Stop),
        (true, ConnStatus::Banned | ConnStatus::Connecting | ConnStatus::Reconnecting) => None,
        (true, _) => Some(Mirror::Stop),
        _ => None,
    }
}

/// V110-M28 Puts the room's anonymous mirror right for its status. The anonymous join doesn't
/// count toward the account's chat limit (`joining` skips it).
pub(super) fn sync_mirror(app: &AppHandle, inner: &mut Inner, key: &RoomKey) {
    let read = app.state::<Chat>().read_banned.load(std::sync::atomic::Ordering::Relaxed);
    let Some(e) = inner.rooms.get_mut(key) else { return };
    match mirror_step(matches!(key.0, Identity::Account(_)), e.room.status, e.mirror, read) {
        Some(Mirror::Start) => {
            e.mirror = true;
            ensure(app, inner, &Identity::Anon).client.join(&key.1);
        }
        Some(Mirror::Stop) => {
            e.mirror = false;
            release_anon(inner, &key.1);
        }
        None => {}
    }
}

/// V110-M28 The setting changed: every account room settles.
pub(super) fn resync_mirrors(app: &AppHandle) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let mut inner = chat.lock();
    let keys: Vec<RoomKey> = inner.rooms.keys().cloned().collect();
    for k in &keys {
        sync_mirror(app, &mut inner, k);
    }
}

/// V110-M28 A banned room reads `channel` through the anonymous connection.
pub(super) fn mirrored_here(inner: &Inner, channel: &str) -> bool {
    inner.rooms.iter().any(|((_, c), e)| e.mirror && c == channel)
}

/// V110-F53 Nothing reads through the anonymous connection: no anonymous room and no mirror.
pub(super) fn anon_idle(inner: &Inner) -> bool {
    idle_without(inner.rooms.iter().map(|((i, _), e)| (i, e.mirror)))
}

/// `rooms` as (identity, is a mirror).
fn idle_without<'a>(mut rooms: impl Iterator<Item = (&'a Identity, bool)>) -> bool {
    !rooms.any(|(i, mirror)| mirror || matches!(i, Identity::Anon))
}

/// V110-M28 A mirror ended: the anonymous connection leaves the channel unless an anonymous
/// room or another mirror still needs it, and stops when it has nothing left.
pub(super) fn release_anon(inner: &mut Inner, channel: &str) {
    let anon_room = inner.rooms.keys().any(|(i, c)| matches!(i, Identity::Anon) && c == channel);
    if !anon_room
        && !mirrored_here(inner, channel)
        && let Some(s) = inner.sessions.get(&Identity::Anon)
    {
        s.client.part(channel);
    }
    if anon_idle(inner) && let Some(s) = inner.sessions.remove(&Identity::Anon) {
        s.client.stop();
    }
}

/// V110-M28 Routes a line from the anonymous connection into the banned rooms of `channel`.
fn mirror(app: &AppHandle, channel: &str, line: &Line<'_>, raw: &str) {
    // Its own join and part (and other anonymous readers') say nothing.
    if matches!(line.command, "JOIN" | "PART") && line.nick().starts_with("justinfan") {
        return;
    }
    let to: Vec<Identity> = app
        .state::<Chat>()
        .lock()
        .rooms
        .iter()
        .filter(|((i, c), e)| e.mirror && c == channel && matches!(i, Identity::Account(_)))
        .map(|((i, _), _)| i.clone())
        .collect();
    for id in &to {
        room_line(app, id, channel, line, raw);
    }
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
        entry.join_retried = false;
        let events = entry.room.set_status(ConnStatus::Joined, now_ms());
        queue(entry, events, &chat.wake);
        let gap = entry.room.take_gap(now_ms());
        sync_mirror(app, &mut inner, &key);
        drop(inner);
        if was_refused {
            crate::connections::notify(app);
        }
        if let Some(span) = gap {
            // V108-C31 FFZ has no live feed: read its room again after a drop.
            crate::emotes::ffz_after_reconnect(app, channel);
            fill_gap(app.clone(), key, span);
        }
        return;
    }
    // C37 Lines asked for before the setting went off, until the next connect. V108-C02 Each
    // kind has its own setting, and crowded chats show neither.
    if !super::presence_shown(app, &chat, line.command, channel) {
        return;
    }
    let role_before = entry.room.role();
    let events = entry.room.handle_line(line, now_ms(), &emotes);
    let refused = entry.room.status == ConnStatus::Refused;
    let role = entry.room.role();
    // U9 Only rooms no view shows collect news.
    let news = if entry.views.is_empty() { unread::news(&events) } else { News::default() };
    // V107-Mr Each new mention also counts, so the tab's number follows.
    // V108-N11 A newer rule colour repaints a mark that is already lit.
    let lit = news.level > entry.unread.level || news.mentions > 0 || news.color.is_some();
    if lit {
        entry.unread = entry.unread.add(news);
    }
    let news = entry.unread;
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
    // V108-N25 Spikes and events this line made.
    let moments = entry.room.take_moments();
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
    sync_mirror(app, &mut inner, &key);
    drop(inner);

    if refused != was_refused {
        crate::connections::notify(app);
    }
    // V102-21 A role change here: mod feeds (1.46) and every window.
    if role != role_before && let Identity::Account(_) = identity {
        super::roles::changed(app);
    }
    if !moments.is_empty() {
        super::moments::announce(app, channel, moments);
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
/// V108-C01 Also what was said while the room was parked.
fn fill_gap(app: AppHandle, key: RoomKey, (after, before, limit): (i64, i64, u32)) {
    let history = app.state::<crate::settings::Settings>().read::<cyex_settings::schema::Chat>(cyex_settings::Partition::Chat).history;
    if !history {
        say(&app, &key, cyex_chat::GAP_NOTE);
        return;
    }
    tauri::async_runtime::spawn(async move {
        let http = app.state::<crate::auth::Auth>().http.clone();
        // V108-C01 A chat on screen (a tab just reopened) doesn't wait behind hidden ones.
        let lane = super::shown::lane(&app, &key.1);
        // A parked room that came back gets its emotes again meanwhile; the fetch doesn't
        // wait for them (V108-C01).
        let fetch = cyex_twitch::recent::recent_between(&http, &key.1, after, before, limit, lane);
        let ready = crate::emotes::ready(&app, &key.1, std::time::Duration::from_secs(4));
        let (got, ()) = tokio::join!(fetch, ready);
        let (lines, recovering) = match got {
            Ok(got) => got,
            Err(e) => {
                eprintln!("chat: couldn't fill the gap in {}: {}", key.1, e.describe("recent-messages"));
                // S2 Like the first history (C35), logs.cyex.app stands in.
                match gap_from_logs(&http, &key.1, after, before, limit).await {
                    Some(lines) => (lines, false),
                    None => {
                        if let Some(note) = gap_failed_note(&e) {
                            say(&app, &key, note);
                        }
                        unfilled(&app, &key, after);
                        return;
                    }
                }
            }
        };
        let emotes = crate::emotes::room(&app, account_of(&key.0), &key.1);
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        let Some(entry) = inner.rooms.get_mut(&key) else {
            // V108-C01 Closed meanwhile: the parked room takes the lines.
            if let Some(room) = inner.parked.get_mut(&key) {
                room.fill_gap(&lines, now_ms(), &emotes);
            }
            return;
        };
        let messages = entry.room.fill_gap(&lines, now_ms(), &emotes);
        if !messages.is_empty() {
            queue(entry, [RoomEvent::Gap { messages }], &chat.wake);
        }
        drop(inner);
        if cyex_chat::gap_may_miss(true, limit, lines.len(), recovering) {
            say(&app, &key, cyex_chat::GAP_NOTE);
        }
    });
}

/// S2 The gap's lines from logs.cyex.app; `None` when it has none or can't be reached.
async fn gap_from_logs(http: &cyex_twitch::Http, login: &str, after: i64, before: i64, limit: u32) -> Option<Vec<String>> {
    let from = cyex_twitch::clock::rfc3339(after / 1000);
    let to = cyex_twitch::clock::rfc3339(before / 1000 + 1);
    cyex_twitch::logs::channel_lines(http, login, limit, &from, &to)
        .await
        .ok()
        .filter(|l| !l.is_empty())
}

/// S2 What a failed gap fill says in chat. Nothing when the service never serves this
/// channel: its first history already said so, and every reopen (a parked room, V108-C01)
/// or reconnect would repeat it. "While disconnected" was wrong for a reopened chat.
fn gap_failed_note(e: &cyex_twitch::Error) -> Option<&'static str> {
    (!cyex_twitch::recent::lasting(e)).then_some("Couldn’t load what was missed")
}

/// V108-C01 A failed fill of a room closed meanwhile: restored, it asks for the span again.
fn unfilled(app: &AppHandle, key: &RoomKey, after: i64) {
    if let Some(room) = app.state::<Chat>().lock().parked.get_mut(key) {
        room.unfilled(after);
    }
}

/// C35 How far back history from logs.cyex.app may go.
const LOGS_HISTORY_SECS: i64 = 24 * 3600;

/// C1 Fetches up to `limit` recent messages in the background and adds them above the
/// live ones. C35 Where the history service has none, logs.cyex.app stands in.
/// V108-C01 Dropped if the room was closed and opened again meanwhile (`epoch`).
pub fn load_history(app: AppHandle, key: RoomKey, limit: u32, epoch: u64) {
    tauri::async_runtime::spawn(async move {
        let http = app.state::<crate::auth::Auth>().http.clone();
        // V108-C01 Hidden or shown, a chat loads all its history lines (Chatterino's way);
        // hidden ones wait in the slower lane.
        let lane = super::shown::lane(&app, &key.1);
        let lines = fetch_history(&app, &http, &key, limit, lane).await;
        let emotes = crate::emotes::room(&app, account_of(&key.0), &key.1);
        let refill = {
            let chat = app.state::<Chat>();
            let mut inner = chat.lock();
            let Some(entry) = inner.rooms.get_mut(&key).filter(|e| e.epoch == epoch) else { return };
            if let Some(lines) = lines {
                entry.history = true;
                let messages = entry.room.history(&lines, now_ms(), &emotes);
                if !messages.is_empty() {
                    queue(entry, [RoomEvent::History { messages }], &chat.wake);
                }
            }
            // V107-M1 Only now may a refill run: it starts above these lines.
            entry.loaded = true;
            super::refill::first_history_in(&entry.room, !entry.views.is_empty())
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
    fn v110_m28_a_banned_room_reads_anonymously_until_it_is_let_in() {
        use ConnStatus::*;
        // Starts on the ban, only with the setting on and only for an account.
        assert_eq!(mirror_step(true, Banned, false, true), Some(Mirror::Start));
        assert_eq!(mirror_step(true, Banned, false, false), None);
        assert_eq!(mirror_step(false, Banned, false, true), None);
        assert_eq!(mirror_step(true, Joined, false, true), None);
        // Trying to get back in keeps it; being in, or the setting going off, ends it.
        for s in [Banned, Connecting, Reconnecting] {
            assert_eq!(mirror_step(true, s, true, true), None, "{s:?}");
        }
        assert_eq!(mirror_step(true, Joined, true, true), Some(Mirror::Stop));
        assert_eq!(mirror_step(true, AuthFailed, true, true), Some(Mirror::Stop));
        assert_eq!(mirror_step(true, Banned, true, false), Some(Mirror::Stop));
    }

    #[test]
    fn v110_f53_the_anonymous_connection_stays_for_a_mirror() {
        let acc = Identity::Account("1".into());
        assert!(idle_without([(&acc, false)].into_iter()));
        assert!(!idle_without([(&acc, true)].into_iter()), "a banned room reads through it");
        assert!(!idle_without([(&Identity::Anon, false)].into_iter()));
        assert!(idle_without(std::iter::empty()));
    }

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
    fn a_gap_the_service_never_serves_stays_quiet() {
        let ignored = cyex_twitch::Error::Api {
            status: 200,
            message: "channel_ignored: ignored".into(),
        };
        assert_eq!(gap_failed_note(&ignored), None);
        let down = cyex_twitch::Error::Api {
            status: 503,
            message: String::new(),
        };
        assert_eq!(gap_failed_note(&down), Some("Couldn’t load what was missed"));
    }

    #[test]
    fn untargeted_notices_go_to_every_chat() {
        assert!(every(":tmi.twitch.tv NOTICE * :x"));
        assert!(every(":tmi.twitch.tv NOTICE #jtv :x"));
        assert!(!every(":tmi.twitch.tv NOTICE #chan :x"));
        assert!(!every(":a!a@a PRIVMSG #chan :x"));
    }
}
