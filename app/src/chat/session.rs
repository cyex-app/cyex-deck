//! IRC sessions: one client per identity and the task that routes its lines to rooms.

use std::sync::Arc;

use cyex_chat::irc::{self, Anonymous, Client, Credentials, Event, Line, LinkStatus, Window};
use cyex_chat::model::{ConnStatus, Flag, RoomEvent};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use super::creds::AccountCreds;
use super::unread::{self, Unread};
use super::{Chat, Identity, Inner, RoomKey, Session, account_of, budget, mention, now_ms, queue};

/// The session for `identity`, started if needed.
pub fn ensure<'a>(app: &AppHandle, inner: &'a mut Inner, identity: &Identity) -> &'a Session {
    inner.sessions.entry(identity.clone()).or_insert_with(|| {
        let creds: Arc<dyn Credentials> = match identity {
            Identity::Anon => Arc::new(Anonymous),
            Identity::Account(id) => Arc::new(AccountCreds::new(app.clone(), id.clone())),
        };
        let (tx, rx) = mpsc::unbounded_channel();
        let (client, task) = Client::new(creds, irc::tls(), tx);
        tauri::async_runtime::spawn(task);
        tauri::async_runtime::spawn(pump(app.clone(), identity.clone(), rx));
        Session {
            client,
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
    }
}

async fn pump(app: AppHandle, identity: Identity, mut rx: mpsc::UnboundedReceiver<Event>) {
    while let Some(event) = rx.recv().await {
        match event {
            Event::Status(s) => link_status(&app, &identity, s),
            Event::Line(raw) => line(&app, &identity, &raw),
        }
    }
}

fn link_status(app: &AppHandle, identity: &Identity, link: LinkStatus) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let now = now_ms();
    for ((id, _), entry) in inner.rooms.iter_mut() {
        if id != identity || entry.room.status == ConnStatus::BudgetFull {
            continue;
        }
        let status = match link {
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
        if let Some(ev) = entry.room.set_status(status) {
            queue(entry, [ev], &chat.wake);
        }
    }
}

fn line(app: &AppHandle, identity: &Identity, raw: &str) {
    let Some(line) = Line::parse(raw) else { return };
    let Some(channel) = line.channel() else { return };
    let emotes = crate::emotes::room(app, account_of(identity), channel);
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let key = (identity.clone(), channel.to_owned());
    let Some(entry) = inner.rooms.get_mut(&key) else { return };

    if line.command == "JOIN" {
        if let Some(ev) = entry.room.set_status(ConnStatus::Joined) {
            queue(entry, [ev], &chat.wake);
        }
        let gap = entry.room.take_gap(now_ms());
        drop(inner);
        if let Some(span) = gap {
            fill_gap(app.clone(), key, span);
        }
        return;
    }
    let was_privileged = entry.room.privileged();
    let events = entry.room.handle_line(&line, now_ms(), &emotes);
    let privileged = entry.room.privileged();
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

    if privileged != was_privileged
        && let Identity::Account(id) = identity
    {
        budget(app, |b| {
            if let Some(b) = b.get_mut(id) {
                b.set_privileged(channel, privileged);
            }
        });
        crate::connections::notify(app);
        // 1.46 A role change here may open or close this account's mod feeds.
        crate::moderation::feeds::sync(app);
    }
    let muted = super::rules::current(app).muted(channel);
    for (m, outcome) in outcomes {
        if (outcome.sound || outcome.flash) && !muted {
            mention::notify(app, channel, &m, &outcome);
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
                eprintln!("chat: couldn't fill the gap in {}: {e}", key.1);
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

/// C1 Fetches recent messages in the background and adds them above the live ones.
pub fn load_history(app: AppHandle, key: RoomKey) {
    tauri::async_runtime::spawn(async move {
        let http = app.state::<crate::auth::Auth>().http.clone();
        let lines = match cyex_twitch::recent::recent_messages(&http, &key.1).await {
            Ok(lines) => lines,
            Err(e) => {
                eprintln!("chat: no history for {}: {e}", key.1);
                return;
            }
        };
        let emotes = crate::emotes::room(&app, account_of(&key.0), &key.1);
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        let Some(entry) = inner.rooms.get_mut(&key) else { return };
        let messages = entry.room.history(&lines, now_ms(), &emotes);
        if !messages.is_empty() {
            queue(entry, [RoomEvent::History { messages }], &chat.wake);
        }
    });
}
