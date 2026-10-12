//! V103-22 Where each channel's regular chat shows, by window: chats in tabs, popped-out
//! chats and `#channel` chats. The mod view's and the stream manager's own chat panels
//! follow it (`chat_shown`, event `chat:shown`). They don't count themselves, so two of
//! them never switch each other on and off; chat overlays (U7) don't count either.
//!
//! Calm by design: changes go out once views stopped changing for `QUIET`, and a window
//! that can show chats counts as loading from when it's made until `GRACE` after its page
//! loaded, so nothing decides while a saved layout's windows are still coming up.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::Chat;

pub const EVENT: &str = "chat:shown";
/// Views opening and closing are sent once they've been still this long.
const QUIET: Duration = Duration::from_millis(300);
/// A window's chats open this soon after its page loaded.
const GRACE: Duration = Duration::from_millis(1000);
/// A window whose page never says it loaded stops counting after this.
const CAP: Duration = Duration::from_secs(10);

/// Where the regular chats show.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatShown {
    /// Channel login → the windows showing its chat, A-Z.
    pub chats: BTreeMap<String, Vec<String>>,
    /// Windows still loading: their chats may open any moment.
    pub loading: Vec<String>,
}

#[derive(Default)]
pub struct Shown {
    state: Mutex<State>,
    /// Goes up with every view change; a send waits until it stops.
    changes: AtomicU64,
}

#[derive(Default)]
struct State {
    loading: Loading,
    sent: Option<ChatShown>,
}

impl Shown {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// Windows that may still open chats, each until when it counts.
#[derive(Debug, Default)]
struct Loading(HashMap<String, Instant>);

impl Loading {
    /// Made or reloading: counts until its page loaded, at most `CAP`.
    fn start(&mut self, label: &str, now: Instant) -> Instant {
        let until = now + CAP;
        self.0.insert(label.to_owned(), until);
        until
    }

    /// Its page loaded: `GRACE` more for its chats. `None` when it wasn't loading.
    fn finish(&mut self, label: &str, now: Instant) -> Option<Instant> {
        let until = self.0.get_mut(label)?;
        *until = (*until).min(now + GRACE);
        Some(*until)
    }

    fn remove(&mut self, label: &str) -> bool {
        self.0.remove(label).is_some()
    }

    fn expire(&mut self, now: Instant) {
        self.0.retain(|_, until| *until > now);
    }

    fn labels(&self) -> Vec<String> {
        let mut out: Vec<String> = self.0.keys().cloned().collect();
        out.sort_unstable();
        out
    }
}

pub fn init(app: &AppHandle) {
    app.manage(Shown::default());
}

/// Login → windows, from views as (login, window). `skip` leaves windows out (overlays).
fn chats_of<'a>(
    views: impl Iterator<Item = (&'a str, &'a str)>,
    skip: impl Fn(&str) -> bool,
) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (login, window) in views.filter(|(_, w)| !skip(w)) {
        out.entry(login.to_owned()).or_default().push(window.to_owned());
    }
    for windows in out.values_mut() {
        windows.sort_unstable();
        windows.dedup();
    }
    out
}

fn chats(app: &AppHandle) -> BTreeMap<String, Vec<String>> {
    let Some(chat) = app.try_state::<Chat>() else {
        return BTreeMap::new();
    };
    let inner = chat.lock();
    chats_of(
        inner
            .views
            .iter()
            .filter(|(id, _)| !inner.follows.contains(id))
            .map(|(_, ((_, login), window))| (login.as_str(), window.as_str())),
        crate::windows::overlay::is_overlay_window,
    )
}

/// Now, for `chat_shown`.
pub fn current(app: &AppHandle) -> ChatShown {
    let Some(shown) = app.try_state::<Shown>() else {
        return ChatShown::default();
    };
    let mut st = shown.lock();
    st.loading.expire(Instant::now());
    ChatShown {
        chats: chats(app),
        loading: st.loading.labels(),
    }
}

/// Sends what's true now, if it differs from what was sent last. Under the lock, so two
/// sends can't cross.
fn send(app: &AppHandle) {
    let Some(shown) = app.try_state::<Shown>() else {
        return;
    };
    let mut st = shown.lock();
    st.loading.expire(Instant::now());
    let now = ChatShown {
        chats: chats(app),
        loading: st.loading.labels(),
    };
    if st.sent.as_ref() == Some(&now) {
        return;
    }
    let _ = app.emit(EVENT, &now);
    st.sent = Some(now);
}

/// A regular chat view opened or closed: sent once views stay still for `QUIET`. Takes no
/// lock, so it's safe to call holding the chat's.
pub fn changed(app: &AppHandle) {
    let Some(shown) = app.try_state::<Shown>() else {
        return;
    };
    let mark = shown.changes.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(QUIET).await;
        if app.state::<Shown>().changes.load(Ordering::Relaxed) == mark {
            send(&app);
        }
    });
}

/// Sends once `until` has passed.
fn send_at(app: &AppHandle, until: Instant) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(until.saturating_duration_since(Instant::now()) + Duration::from_millis(5)).await;
        send(&app);
    });
}

/// A window that can show chats is being made, or its page starts again.
pub fn loading(app: &AppHandle, label: &str) {
    let Some(shown) = app.try_state::<Shown>() else {
        return;
    };
    let until = shown.lock().loading.start(label, Instant::now());
    send(app);
    send_at(app, until);
}

/// Its page loaded.
pub fn loaded(app: &AppHandle, label: &str) {
    let Some(shown) = app.try_state::<Shown>() else {
        return;
    };
    let until = shown.lock().loading.finish(label, Instant::now());
    if let Some(until) = until {
        send_at(app, until);
    }
}

/// The window closed.
pub fn gone(app: &AppHandle, label: &str) {
    let Some(shown) = app.try_state::<Shown>() else {
        return;
    };
    let was = shown.lock().loading.remove(label);
    if was {
        send(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chats_by_channel_without_overlays() {
        let views = [
            ("xqc", "main"),
            ("xqc", "panel-chat_t1"),
            ("xqc", "main"),
            ("xqc", "panel-overlay_t1"),
            ("jynxzi", "panel-overlay_t2"),
            ("papaplatte", "panel-tab_a"),
        ];
        let out = chats_of(views.into_iter(), |w| w.starts_with("panel-overlay"));
        assert_eq!(out.len(), 2, "a channel only in an overlay isn't listed");
        assert_eq!(out["xqc"], ["main", "panel-chat_t1"]);
        assert_eq!(out["papaplatte"], ["panel-tab_a"]);
    }

    #[test]
    fn a_window_counts_as_loading_until_a_moment_after_its_page() {
        let t = Instant::now();
        let mut l = Loading::default();
        assert_eq!(l.start("panel-tab_a", t), t + CAP);
        l.start("main", t);
        assert_eq!(l.labels(), ["main", "panel-tab_a"]);
        // Loaded: a second more, never longer than it had.
        assert_eq!(
            l.finish("panel-tab_a", t + Duration::from_millis(500)),
            Some(t + Duration::from_millis(1500))
        );
        assert_eq!(
            l.finish("panel-usercard_x", t),
            None,
            "a window that wasn't loading"
        );
        l.expire(t + Duration::from_millis(1499));
        assert_eq!(l.labels(), ["main", "panel-tab_a"]);
        l.expire(t + Duration::from_millis(1500));
        assert_eq!(l.labels(), ["main"]);
        // Never loaded: it stops counting after the cap.
        l.expire(t + CAP);
        assert!(l.labels().is_empty());
        // Reloading starts it again; a closed window goes at once.
        l.start("main", t);
        assert!(l.remove("main") && !l.remove("main"));
    }
}
