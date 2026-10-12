//! 1.26 Whispers for the default account. With a GQL sign-in: every thread and its history
//! from Twitch, live over Hermes (what you send from other devices too), read state kept in
//! step with Twitch. With only a Helix sign-in: whispers that arrive while the app runs
//! (EventSub) and sending. Lines draw like chat (global emotes, readable names) and can
//! also show inline in every open chat (Settings → Notifications).

mod live;
mod send;
pub mod view;

pub use live::on_hermes;
pub use send::{reply_target, send};

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use cyex_chat::build::sender::default_color;
use cyex_chat::build::whisper::{self as line, From, Whisper};
use cyex_chat::build::{Colors, Me as ChatMe};
use cyex_chat::model::{ChatMessage, plain};
use cyex_emotes::RoomEmotes;
use cyex_eventsub::session::Session;
use cyex_twitch::gql::{self, whispers as api};
use cyex_twitch::hermes::Kind;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::auth;
use view::{WhisperConversation, WhisperEdit, WhisperPage, WhisperState, WhisperThread, WhisperUser};

const CHANGED: &str = "whispers:changed";
const LINE: &str = "whispers:line";
const REFRESH: &str = "whispers:refresh";
/// Lines kept per thread from this run: all there is without history, and how echoes of
/// what we sent are recognized.
const RECENT: usize = 200;
const READ_SCOPE: &str = "user:read:whispers";
const NO_ACCOUNT: &str = "Sign in to whisper (Settings → Accounts).";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Me {
    id: String,
    login: String,
    display: String,
    /// GQL sign-in: history, Hermes, GQL sending.
    gql: bool,
}

struct ThreadState {
    view: WhisperThread,
    /// The newest message's id, for marking it read.
    last_id: Option<String>,
}

#[derive(Default)]
struct Inner {
    me: Option<Me>,
    threads: Vec<ThreadState>,
    cursor: Option<String>,
    loading: bool,
    block_strangers: bool,
    /// Who whispered last, for `/r`.
    last_from: Option<String>,
    /// The thread on screen in a focused window: no unread count, no alerts.
    viewing: Option<String>,
    recent: HashMap<String, VecDeque<Arc<ChatMessage>>>,
    /// EventSub for accounts with only Helix. Dropping it ends the session.
    eventsub: Option<Session>,
    colors: Colors,
}

#[derive(Default)]
pub struct Whispers(Mutex<Inner>);

pub fn init(app: &AppHandle) {
    app.manage(Whispers::default());
    accounts_changed(app);
}

fn lock(app: &AppHandle) -> MutexGuard<'_, Inner> {
    app.state::<Whispers>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// Accounts changed: follow the default account.
pub fn accounts_changed(app: &AppHandle) {
    if app.try_state::<Whispers>().is_none() {
        return;
    }
    let accounts = auth::accounts(app);
    let wanted = accounts.default_account().and_then(|a| {
        let gql = auth::gql_token(app, &a.user_id).is_some();
        let helix = auth::helix_scoped(app, &a.user_id, READ_SCOPE).is_some();
        (gql || helix).then(|| Me {
            id: a.user_id.clone(),
            login: a.login.clone(),
            display: if a.display_name.is_empty() {
                a.login.clone()
            } else {
                a.display_name.clone()
            },
            gql,
        })
    });
    let old = {
        let mut inner = lock(app);
        if inner.me == wanted {
            return;
        }
        let old = inner.me.take();
        *inner = Inner {
            me: wanted.clone(),
            ..Inner::default()
        };
        old
    };
    if let Some(old) = old.filter(|o| o.gql) {
        crate::hermes::remove_user(app, &old.id, Kind::Whispers);
    }
    if let Some(me) = wanted {
        if me.gql {
            crate::hermes::add_user(app, &me.id, Kind::Whispers);
            load_threads(app.clone(), false);
        } else {
            live::start_eventsub(app, &me.id);
        }
    }
    emit_changed(app);
}

pub fn state(app: &AppHandle) -> WhisperState {
    let inner = lock(app);
    let account = inner.me.as_ref().map(|m| WhisperUser {
        id: m.id.clone(),
        login: m.login.clone(),
        name: m.display.clone(),
        color: cyex_chat::model::NameColor {
            dark: String::new(),
            light: String::new(),
        },
        avatar: None,
    });
    WhisperState {
        history: inner.me.as_ref().is_some_and(|m| m.gql),
        account,
        threads: inner.threads.iter().map(|t| t.view.clone()).collect(),
        more: inner.cursor.is_some(),
        loading: inner.loading,
        block_strangers: inner.block_strangers,
        unread: inner
            .threads
            .iter()
            .filter(|t| !t.view.muted && !t.view.archived)
            .map(|t| t.view.unread)
            .sum(),
    }
}

fn emit_changed(app: &AppHandle) {
    let _ = app.emit(CHANGED, state(app));
}

/// Loads the first page of threads, or the next one.
pub fn load_threads(app: AppHandle, more: bool) {
    let (me, after) = {
        let mut inner = lock(&app);
        let Some(me) = inner.me.clone().filter(|m| m.gql) else {
            return;
        };
        if inner.loading || (more && inner.cursor.is_none()) {
            return;
        }
        inner.loading = true;
        (me, if more { inner.cursor.clone() } else { None })
    };
    tauri::async_runtime::spawn(async move {
        let result = match GqlCall::new(&app, &me.id) {
            Ok(c) => api::threads(&c.http, c.auth(), &me.id, after.as_deref()).await,
            Err(e) => Err(e),
        };
        {
            let mut inner = lock(&app);
            inner.loading = false;
            if inner.me.as_ref() != Some(&me) {
                return;
            }
            match result {
                Ok(page) => {
                    inner.cursor = page.cursor;
                    inner.block_strangers = page.block_strangers;
                    for t in &page.threads {
                        upsert(&mut inner, &me, t);
                    }
                    sort(&mut inner);
                }
                Err(e) => eprintln!("whispers: threads: {e}"),
            }
        }
        emit_changed(&app);
    });
}

/// A conversation with `login`, loaded fresh. A new one if they never whispered.
pub async fn open(app: &AppHandle, login: &str) -> Result<WhisperConversation, String> {
    let me = current(app)?;
    let other = send::resolve(app, login).await?;
    let thread_id = api::thread_id(&me.id, &other.id);
    if me.gql {
        let c = GqlCall::new(app, &me.id).map_err(|e| failed(&e))?;
        let page = api::thread(&c.http, c.auth(), &me.id, &thread_id, api::MESSAGES_PAGE, None)
            .await
            .map_err(|e| failed(&e))?;
        if let Some((t, msgs)) = page {
            let emotes = crate::emotes::global(app);
            let mut inner = lock(app);
            upsert(&mut inner, &me, &t);
            if let Some(newest) = msgs.messages.first() {
                set_last(&mut inner, &t.id, newest);
            }
            sort(&mut inner);
            let other_login = t.other.login.clone();
            let lines: Vec<ChatMessage> = msgs
                .messages
                .iter()
                .rev()
                .map(|m| build(&mut inner, &me, &emotes, m, &other_login))
                .collect();
            let recent = inner.recent.entry(t.id.clone()).or_default();
            recent.clear();
            recent.extend(lines.iter().cloned().map(Arc::new));
            trim(recent);
            let view = find(&inner, &t.id)
                .map(|s| s.view.clone())
                .unwrap_or_else(|| blank(&mut inner.colors, &t.id, &t.other));
            drop(inner);
            emit_changed(app);
            return Ok(WhisperConversation {
                thread: view,
                messages: lines,
                cursor: msgs.cursor,
            });
        }
    }
    let mut inner = lock(app);
    let view = find(&inner, &thread_id)
        .map(|s| s.view.clone())
        .unwrap_or_else(|| blank(&mut inner.colors, &thread_id, &other));
    let messages = inner
        .recent
        .get(&thread_id)
        .map(|r| r.iter().map(|m| (**m).clone()).collect())
        .unwrap_or_default();
    Ok(WhisperConversation {
        thread: view,
        messages,
        cursor: None,
    })
}

/// Older lines of a thread.
pub async fn older(app: &AppHandle, thread_id: &str, cursor: &str) -> Result<WhisperPage, String> {
    let me = current(app)?;
    let c = GqlCall::new(app, &me.id).map_err(|e| failed(&e))?;
    let page = api::thread(
        &c.http,
        c.auth(),
        &me.id,
        thread_id,
        api::MESSAGES_PAGE,
        Some(cursor),
    )
    .await
    .map_err(|e| failed(&e))?;
    let Some((t, msgs)) = page else {
        return Ok(WhisperPage {
            messages: Vec::new(),
            cursor: None,
        });
    };
    let emotes = crate::emotes::global(app);
    let mut inner = lock(app);
    let login = t.other.login.clone();
    let messages = msgs
        .messages
        .iter()
        .rev()
        .map(|m| build(&mut inner, &me, &emotes, m, &login))
        .collect();
    Ok(WhisperPage {
        messages,
        cursor: msgs.cursor,
    })
}

/// Marks a thread read, here and on Twitch.
pub async fn mark_read(app: &AppHandle, thread_id: &str) -> Result<(), String> {
    let (me, last) = {
        let mut inner = lock(app);
        let me = inner.me.clone().ok_or(NO_ACCOUNT)?;
        let Some(t) = inner.threads.iter_mut().find(|t| t.view.id == thread_id) else {
            return Ok(());
        };
        let had = t.view.unread;
        t.view.unread = 0;
        (me, t.last_id.clone().filter(|_| had > 0))
    };
    emit_changed(app);
    let (Some(last), true) = (last, me.gql) else {
        return Ok(());
    };
    if last.starts_with("local-") {
        return Ok(());
    }
    let c = GqlCall::new(app, &me.id).map_err(|e| failed(&e))?;
    let edit = api::ThreadEdit {
        read_up_to: Some(&last),
        ..Default::default()
    };
    api::edit_thread(&c.http, c.auth(), thread_id, &edit)
        .await
        .map_err(|e| failed(&e))
}

/// Archive, mute, spam or take back access. Saved on Twitch first.
pub async fn edit(app: &AppHandle, thread_id: &str, e: WhisperEdit) -> Result<(), String> {
    let me = current(app)?;
    if !me.gql {
        return Err("Thread options need a GQL sign-in.".into());
    }
    let c = GqlCall::new(app, &me.id).map_err(|e| failed(&e))?;
    let edit = api::ThreadEdit {
        read_up_to: None,
        archived: e.archived,
        muted: e.muted,
        spam: e.spam,
        revoke_access: e.revoke.unwrap_or(false),
    };
    api::edit_thread(&c.http, c.auth(), thread_id, &edit)
        .await
        .map_err(|e| failed(&e))?;
    {
        let mut inner = lock(app);
        if let Some(t) = inner.threads.iter_mut().find(|t| t.view.id == thread_id) {
            if let Some(v) = e.archived {
                t.view.archived = v;
            }
            if let Some(v) = e.muted {
                t.view.muted = v;
            }
            if e.spam.is_some() {
                t.view.spam = false;
            }
            if e.revoke == Some(true) {
                t.view.temporary = false;
            }
        }
    }
    emit_changed(app);
    Ok(())
}

pub async fn read_all(app: &AppHandle) -> Result<(), String> {
    let me = current(app)?;
    if me.gql {
        let c = GqlCall::new(app, &me.id).map_err(|e| failed(&e))?;
        api::mark_all_read(&c.http, c.auth())
            .await
            .map_err(|e| failed(&e))?;
    }
    for t in lock(app).threads.iter_mut() {
        t.view.unread = 0;
    }
    emit_changed(app);
    Ok(())
}

pub async fn block_strangers(app: &AppHandle, on: bool) -> Result<(), String> {
    let me = current(app)?;
    if !me.gql {
        return Err("This needs a GQL sign-in.".into());
    }
    let c = GqlCall::new(app, &me.id).map_err(|e| failed(&e))?;
    api::block_strangers(&c.http, c.auth(), on)
        .await
        .map_err(|e| failed(&e))?;
    lock(app).block_strangers = on;
    emit_changed(app);
    Ok(())
}

/// Blocks the other person on Twitch, from the default account.
pub async fn block(app: &AppHandle, login: &str) -> Result<(), String> {
    let me = current(app)?;
    crate::chat::run::block_user(app, &me.id, login, true).await
}

/// The thread a focused view shows, or `None`.
pub fn viewing(app: &AppHandle, thread_id: Option<String>) {
    lock(app).viewing = thread_id;
}

fn current(app: &AppHandle) -> Result<Me, String> {
    lock(app).me.clone().ok_or_else(|| NO_ACCOUNT.into())
}

/// The account's GQL token, owned, for one or more calls.
pub(crate) struct GqlCall {
    pub http: cyex_twitch::Http,
    token: String,
    client_id: String,
}

impl GqlCall {
    pub fn new(app: &AppHandle, account: &str) -> cyex_twitch::Result<Self> {
        let Some((token, client_id)) = auth::gql_token(app, account) else {
            return Err(cyex_twitch::Error::Api {
                status: 401,
                message: "unauthenticated".into(),
            });
        };
        Ok(Self {
            http: app.state::<auth::Auth>().http.clone(),
            token,
            client_id,
        })
    }

    pub fn auth(&self) -> gql::Auth<'_> {
        gql::Auth {
            client_id: &self.client_id,
            token: Some(&self.token),
        }
    }
}

fn failed(e: &cyex_twitch::Error) -> String {
    if e.is_network() {
        "Couldn't reach Twitch.".into()
    } else {
        eprintln!("whispers: {e}");
        "Twitch didn't take that. Try again in a bit.".into()
    }
}

fn find<'a>(inner: &'a Inner, thread_id: &str) -> Option<&'a ThreadState> {
    inner.threads.iter().find(|t| t.view.id == thread_id)
}

fn user(colors: &mut Colors, p: &api::Person) -> WhisperUser {
    let raw = if p.color.is_empty() {
        default_color(&p.id, &p.login)
    } else {
        &p.color
    };
    WhisperUser {
        id: p.id.clone(),
        login: p.login.clone(),
        name: if p.display.is_empty() {
            p.login.clone()
        } else {
            p.display.clone()
        },
        color: colors.get(raw),
        avatar: p.avatar.clone(),
    }
}

/// A thread with nothing in it yet.
fn blank(colors: &mut Colors, id: &str, other: &api::Person) -> WhisperThread {
    WhisperThread {
        id: id.to_owned(),
        other: user(colors, other),
        unread: 0,
        archived: false,
        muted: false,
        spam: false,
        temporary: false,
        last_text: String::new(),
        last_own: false,
        last_ts: 0,
    }
}

/// Takes Twitch's view of a thread; keeps a known avatar if Twitch sent none.
fn upsert(inner: &mut Inner, me: &Me, t: &api::Thread) {
    let mut view = blank(&mut inner.colors, &t.id, &t.other);
    view.unread = t.unread;
    view.archived = t.archived;
    view.muted = t.muted;
    view.spam = t.spam;
    view.temporary = t.temporary;
    if let Some(last) = &t.last {
        view.last_text = if last.deleted {
            String::new()
        } else {
            last.text.clone()
        };
        view.last_own = last.from.id == me.id;
        view.last_ts = ms(&last.sent_at);
    }
    let last_id = t.last.as_ref().map(|m| m.id.clone());
    match inner.threads.iter_mut().find(|s| s.view.id == t.id) {
        Some(s) => {
            if view.other.avatar.is_none() {
                view.other.avatar = s.view.other.avatar.take();
            }
            s.view = view;
            s.last_id = last_id.or(s.last_id.take());
        }
        None => inner.threads.push(ThreadState { view, last_id }),
    }
}

fn set_last(inner: &mut Inner, thread_id: &str, m: &api::Message) {
    if let Some(t) = inner.threads.iter_mut().find(|t| t.view.id == thread_id) {
        t.last_id = Some(m.id.clone());
    }
}

fn sort(inner: &mut Inner) {
    inner.threads.sort_by_key(|t| std::cmp::Reverse(t.view.last_ts));
}

/// A whisper as a chat line. `other_login`: the other person, named on our own lines.
fn build(
    inner: &mut Inner,
    me: &Me,
    emotes: &RoomEmotes,
    m: &api::Message,
    other_login: &str,
) -> ChatMessage {
    let own = m.from.id == me.id;
    let ranges: Vec<(String, usize, usize)> = m.emotes.iter().map(|e| (e.id.clone(), e.from, e.to)).collect();
    let names = ChatMe::new(&me.id, &me.login, &me.display).names;
    let mut msg = line::message(
        &Whisper {
            id: &m.id,
            ts: ms(&m.sent_at),
            from: From {
                id: &m.from.id,
                login: &m.from.login,
                display: &m.from.display,
                color: &m.from.color,
            },
            text: &m.text,
            emotes: &ranges,
            own,
            deleted: m.deleted,
        },
        &names,
        &mut inner.colors,
        emotes,
    );
    if own {
        msg.meta.whisper_to = Some(other_login.to_owned());
    }
    msg
}

/// Keeps a line for the thread. `false` when it's already there (an echo).
fn remember(inner: &mut Inner, thread_id: &str, m: Arc<ChatMessage>) -> bool {
    let recent = inner.recent.entry(thread_id.to_owned()).or_default();
    if recent.iter().any(|x| x.id == m.id) {
        return false;
    }
    recent.push_back(m);
    trim(recent);
    true
}

fn trim(recent: &mut VecDeque<Arc<ChatMessage>>) {
    while recent.len() > RECENT {
        recent.pop_front();
    }
}

/// RFC 3339 to unix ms; now when it doesn't read.
fn ms(s: &str) -> i64 {
    OffsetDateTime::parse(s, &Rfc3339)
        .map(|t| (t.unix_timestamp_nanos() / 1_000_000) as i64)
        .unwrap_or_else(|_| crate::chat::now_ms())
}

fn emit_line(app: &AppHandle, thread_id: &str, m: &ChatMessage) {
    let line = view::WhisperLine {
        thread_id: thread_id.to_owned(),
        message: m.clone(),
    };
    let _ = app.emit(LINE, line);
}

fn emit_refresh(app: &AppHandle, thread_id: &str) {
    let _ = app.emit(REFRESH, json!({ "threadId": thread_id }));
}

/// Plain text of a line, for previews and toasts.
fn preview(m: &ChatMessage) -> String {
    plain(&m.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_read() {
        assert_eq!(ms("2026-10-10T10:00:00Z"), 1_791_626_400_000);
        assert_eq!(ms("2026-10-10T10:00:00.5Z"), 1_791_626_400_500);
    }

    #[test]
    fn echoes_are_skipped() {
        let mut inner = Inner::default();
        let m = Arc::new(cyex_chat::build::system(
            cyex_chat::model::NoticeKind::Info,
            "x",
            1,
        ));
        assert!(remember(&mut inner, "1_2", m.clone()));
        assert!(!remember(&mut inner, "1_2", m));
    }
}
