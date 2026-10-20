//! New whispers as they happen: Hermes `whispers.<id>` with a GQL sign-in (sent ones from
//! any device too, edits, read state), EventSub with only Helix (received ones). Each new
//! line updates its thread, reaches the open view, shows inline in chat when that's on,
//! and alerts when the user isn't already looking at it.

use std::sync::Arc;

use cyex_eventsub::events::parse_whisper;
use cyex_eventsub::session::{self, Out};
use cyex_eventsub::subs;
use cyex_settings::Partition;
use cyex_settings::schema::Notifications;
use cyex_twitch::gql::whispers::{self as api, Live};
use cyex_twitch::{helix, hermes};
use tauri::{AppHandle, Manager};

use super::{
    GqlCall, build, emit_changed, emit_line, emit_refresh, lock, preview, remember, set_last, sort, upsert,
};
use crate::auth;
use crate::settings::Settings;
use crate::toasts::{self, Open, Pop};

pub fn on_hermes(app: &AppHandle, e: &hermes::Event) {
    let Some(live) = api::from_hermes(&e.payload) else {
        return;
    };
    match live {
        Live::Message {
            thread_id,
            message,
            to,
        } => incoming(app, &thread_id, *message, to),
        Live::Edited { thread_id, .. } | Live::Deleted { thread_id, .. } => {
            refresh(app, thread_id.clone());
            emit_refresh(app, &thread_id);
        }
        Live::Thread { thread_id } => refresh(app, thread_id),
        Live::AllRead => {
            for t in lock(app).threads.iter_mut() {
                t.view.unread = 0;
            }
            emit_changed(app);
        }
    }
}

/// A whisper sent or received. `to` names the other person on our own lines when known.
pub(super) fn incoming(app: &AppHandle, thread_id: &str, m: api::Message, to: Option<api::Person>) {
    let n: Notifications = app.state::<Settings>().read(Partition::Notifications);
    let (line, own, alert, feed_from) = {
        let mut inner = lock(app);
        let Some(me) = inner.me.clone() else { return };
        if !thread_id.split('_').any(|p| p == me.id) {
            return; // an old session's event after the account changed
        }
        let own = m.from.id == me.id;
        let other = if own { to } else { Some(m.from.clone()) };
        // V106-07 Received whispers also go to the Whispers feed.
        let feed_from = if own { None } else { other.clone() };
        let known = inner.threads.iter().position(|t| t.view.id == thread_id);
        let other_login = known
            .map(|i| inner.threads[i].view.other.login.clone())
            .or_else(|| other.as_ref().map(|p| p.login.clone()))
            .unwrap_or_default();
        let emotes = crate::emotes::global(app);
        let line = Arc::new(build(&mut inner, &me, &emotes, &m, &other_login));
        if !remember(&mut inner, thread_id, line.clone()) {
            return;
        }
        let viewing = inner.viewing.as_deref() == Some(thread_id);
        if known.is_none()
            && let Some(p) = &other
        {
            let t = api::Thread {
                id: thread_id.to_owned(),
                other: p.clone(),
                unread: 0,
                archived: false,
                muted: false,
                spam: false,
                temporary: false,
                last: None,
            };
            upsert(&mut inner, &me, &t);
        }
        set_last(&mut inner, thread_id, &m);
        let mut muted = false;
        let mut target = None;
        if let Some(t) = inner.threads.iter_mut().find(|t| t.view.id == thread_id) {
            t.view.last_text = preview(&line);
            t.view.last_own = own;
            t.view.last_ts = line.ts;
            t.view.archived = false;
            if !own && !viewing {
                t.view.unread += 1;
            }
            muted = t.view.muted;
            target = Some((
                t.view.other.login.clone(),
                t.view.other.name.clone(),
                t.view.other.avatar.clone(),
            ));
        }
        sort(&mut inner);
        if !own {
            inner.last_from = Some(m.from.login.clone());
        }
        let alert = (!own && !muted)
            .then_some((viewing, target))
            .filter(|(v, _)| !(*v && toasts::app_in_front(app)));
        (line, own, alert, feed_from)
    };
    if own && lock(app).threads.iter().all(|t| t.view.id != thread_id) {
        refresh(app, thread_id.to_owned()); // sent elsewhere to someone we didn't know
    }
    emit_line(app, thread_id, &line);
    // U4 Streamer mode keeps whispers off chat and the screen.
    let hidden = crate::streamer::hides(app, |s| s.hide_whispers);
    // R8 The whispers feed too.
    if let Some(p) = feed_from.as_ref().filter(|_| !hidden) {
        crate::feeds::whispers::received(app, thread_id, p, &line);
    }
    emit_changed(app);
    if n.whispers.inline && !hidden {
        crate::chat::whisper_line(app, line.clone());
    }
    let Some((_, Some((login, name, avatar)))) = alert.filter(|_| !hidden) else {
        return;
    };
    if n.whispers.sound {
        crate::sound::alert(app, None);
    }
    if n.whispers.flash {
        toasts::flash(app);
    }
    if n.whispers.toast && !toasts::app_in_front(app) {
        toasts::show(
            app,
            Pop {
                title: name,
                text: preview(&line),
                avatar,
                open: Open::Whisper { login },
            },
        );
    }
}

/// Reads one thread's state from Twitch again (read, archived, muted, who's in it).
fn refresh(app: &AppHandle, thread_id: String) {
    let Some(me) = lock(app).me.clone().filter(|m| m.gql) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Ok(c) = GqlCall::new(&app, &me.id) else { return };
        match api::thread(&c.http, c.auth(), &me.id, &thread_id, 1, None).await {
            Ok(Some((t, msgs))) => {
                let mut inner = lock(&app);
                if inner.me.as_ref() != Some(&me) {
                    return;
                }
                upsert(&mut inner, &me, &t);
                if let Some(m) = msgs.messages.first() {
                    set_last(&mut inner, &t.id, m);
                }
                sort(&mut inner);
            }
            Ok(None) => return,
            Err(e) => {
                eprintln!("whispers: refresh: {e}");
                return;
            }
        }
        emit_changed(&app);
    });
}

/// EventSub for an account with only a Helix sign-in: received whispers only.
pub(super) fn start_eventsub(app: &AppHandle, me_id: &str) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (session, task) = session::start(me_id, tx);
    tauri::async_runtime::spawn(task);
    lock(app).eventsub = Some(session);
    let (app, me_id) = (app.clone(), me_id.to_owned());
    tauri::async_runtime::spawn(async move {
        while let Some(out) = rx.recv().await {
            match out {
                Out::Welcome(session_id) => subscribe(&app, &me_id, &session_id).await,
                Out::Notification { kind, event, .. } => {
                    let Some(w) = parse_whisper(&kind, &event) else {
                        continue;
                    };
                    let m = api::Message {
                        id: w.id,
                        nonce: None,
                        sent_at: cyex_twitch::clock::rfc3339(auth::unix_now()),
                        from: api::Person {
                            id: w.from.id.clone(),
                            login: w.from.login,
                            display: w.from.name,
                            color: String::new(),
                            avatar: None,
                        },
                        text: w.text,
                        emotes: Vec::new(),
                        edited: false,
                        deleted: false,
                    };
                    incoming(&app, &api::thread_id(&me_id, &w.from.id), m, None);
                }
                Out::Revoked { kind, .. } => eprintln!("whispers: lost {kind}"),
                Out::Lost => {}
            }
        }
    });
}

async fn subscribe(app: &AppHandle, me_id: &str, session_id: &str) {
    let Some((token, client_id)) = auth::helix_token(app, me_id) else {
        return;
    };
    let http = app.state::<auth::Auth>().http.clone();
    let a = helix::Auth {
        client_id: &client_id,
        token: &token,
    };
    let (kind, version, cond) = subs::whispers(me_id);
    if let Err(e) = helix::eventsub::subscribe(&http, a, session_id, kind, version, &cond).await {
        eprintln!("whispers: couldn't subscribe: {e}");
    }
}
