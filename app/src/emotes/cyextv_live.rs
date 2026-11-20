//! V112-06 Live cyex.tv changes: its event socket follows every open channel (`channel:twitch:<id>`),
//! the set each channel shows and the global set. A set that changed is read again and swapped in
//! (an event only says which set and emote), and the difference is told in chat as for 7TV (C10).
//! A channel switching sets reads the channel again. The topics follow the sets held, so they go
//! when a chat leaves or the provider is switched off. `user:` and `cosmetics` topics are for later
//! (the socket takes them).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cyex_chat::model::Provider;
use cyex_emotes::cyextv::api;
use cyex_emotes::cyextv::events::Event;
use cyex_emotes::cyextv::socket::{EventApi, Topic};
use cyex_emotes::seventv::events::SetChange;
use cyex_emotes::{Emote, EmoteSet, Scope};
use cyex_twitch::pace::Lane;
use cyex_twitch::ws;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

use super::providers::{self, Which};
use super::{Channel, Emotes, Inner, live, load, retry};
use crate::auth::Auth;

/// Events about one set that come within this are one read.
const GATHER: Duration = Duration::from_millis(300);

/// The socket (as many connections as the topics need) and what it follows.
pub struct Live {
    pool: ws::Pool<EventApi>,
    /// The global set's topic (its cyex.tv id).
    global: Mutex<Option<String>>,
    /// Sets (deck ids) with a read waiting or running; more events for them wait for it.
    pending: Mutex<HashSet<String>>,
}

impl Live {
    pub fn new(pool: ws::Pool<EventApi>) -> Self {
        Self {
            pool,
            global: Mutex::new(None),
            pending: Mutex::new(HashSet::new()),
        }
    }

    /// A joined channel's own topic: its active set switching.
    pub(super) fn add_channel(&self, twitch_id: &str) {
        self.pool.add(Topic::Channel(twitch_id.to_owned()));
    }

    pub(super) fn remove_channel(&self, twitch_id: &str) {
        self.pool.remove(&Topic::Channel(twitch_id.to_owned()));
    }

    /// The topic of the global set held now. Call under the Emotes lock with its sets.
    pub(super) fn sync_global(&self, globals: &[Arc<EmoteSet>]) {
        let now = globals
            .iter()
            .find(|s| s.provider == Provider::Cyextv)
            .map(|s| api::set_id(&s.id).to_owned());
        let mut held = self.global.lock().unwrap_or_else(|p| p.into_inner());
        if *held == now {
            return;
        }
        if let Some(n) = &now {
            self.pool.add(Topic::Set(n.clone()));
        }
        if let Some(old) = held.take() {
            self.pool.remove(&Topic::Set(old));
        }
        *held = now;
    }

    /// The channel's set `new` in place of its cyex.tv one, topics included: the new topic goes on
    /// before the old comes off, so the same set stays followed. Call it before the sets are
    /// replaced, under the Emotes lock.
    pub(super) fn swap_channel(&self, c: &Channel, new: Option<&Arc<EmoteSet>>) {
        if let Some(s) = new {
            self.pool.add(Topic::Set(api::set_id(&s.id).to_owned()));
        }
        self.drop_sets(c, Scope::Channel);
    }

    /// The topics of the channel's cyex.tv sets of `scope`.
    pub(super) fn drop_sets(&self, c: &Channel, scope: Scope) {
        for s in c.sets.iter().filter(|s| s.provider == Provider::Cyextv && s.scope == scope) {
            self.pool.remove(&Topic::Set(api::set_id(&s.id).to_owned()));
        }
    }

    /// The first caller for `set` after the last read finished.
    fn claim(&self, set: &str) -> bool {
        self.pending.lock().unwrap_or_else(|p| p.into_inner()).insert(set.to_owned())
    }

    fn release(&self, set: &str) {
        self.pending.lock().unwrap_or_else(|p| p.into_inner()).remove(set);
    }
}

pub fn spawn(app: AppHandle, mut rx: UnboundedReceiver<Event>) {
    tauri::async_runtime::spawn(async move {
        while let Some(e) = rx.recv().await {
            handle(&app, e);
        }
    });
}

fn handle(app: &AppHandle, e: Event) {
    match e {
        Event::Set { set_id, .. } | Event::Personal { set_id } => reload(app, api::deck_set_id(&set_id)),
        Event::ActiveSet { twitch_id, set_id } => switched(app, twitch_id, set_id),
        // An emote's own data changed: its sets say so with their own events.
        Event::EmoteUpdated { .. } => {}
        Event::Resync => resync(app),
    }
}

/// Whether any chat, the global sets or a person holds the set.
fn holds(inner: &Inner, deck: &str) -> bool {
    inner.personal_sets.contains_key(deck)
        || inner.globals.iter().any(|s| s.provider == Provider::Cyextv && s.id == deck)
        || inner
            .channels
            .values()
            .any(|c| c.sets.iter().any(|s| s.provider == Provider::Cyextv && s.id == deck))
}

/// A set changed: read it again (once for a burst of events) and swap it in where it is held.
fn reload(app: &AppHandle, deck: String) {
    let state = app.state::<Emotes>();
    {
        let inner = state.lock();
        if inner.sets_off.cyextv || !holds(&inner, &deck) {
            return;
        }
    }
    if !state.cyextv_live.claim(&deck) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(GATHER).await;
        // Events from here on start another read.
        app.state::<Emotes>().cyextv_live.release(&deck);
        let http = app.state::<Auth>().http.clone();
        let got = api::set_raw(&http, api::set_id(&deck), Lane::Back)
            .await
            .map_err(|e| e.describe("cyex.tv"))
            .and_then(|v| api::parse_set(&v, Scope::Channel).map_err(|e| format!("unexpected answer ({e})")));
        match got {
            Ok(fresh) => apply_reload(&app, &deck, &fresh),
            Err(e) => eprintln!("emotes: no cyex.tv set {deck}: {e}"),
        }
    });
}

fn with_scope(set: &EmoteSet, scope: Scope) -> EmoteSet {
    EmoteSet {
        id: set.id.clone(),
        provider: set.provider,
        scope,
        emotes: set.emotes.clone(),
        name: set.name.clone(),
    }
}

/// What changed from `old` to `new`: emotes added and removed, one emote under another name
/// (same id, new alias) as renamed, and the set's own name.
pub(super) fn diff(old: &EmoteSet, new: &EmoteSet) -> SetChange {
    let key = |e: &Arc<Emote>| (e.id.clone(), e.name.clone());
    let had: HashSet<(String, String)> = old.emotes.iter().map(key).collect();
    let has: HashSet<(String, String)> = new.emotes.iter().map(key).collect();
    let mut removed: Vec<(String, String)> = old.emotes.iter().map(key).filter(|k| !has.contains(k)).collect();
    let mut added: Vec<Emote> = new
        .emotes
        .iter()
        .filter(|e| !had.contains(&key(e)))
        .map(|e| (**e).clone())
        .collect();
    let mut renamed = Vec::new();
    removed.retain(|(id, name)| match added.iter().position(|e| e.id == *id) {
        Some(at) => {
            renamed.push((id.clone(), name.clone(), added.remove(at)));
            false
        }
        None => true,
    });
    let set_renamed = match (&old.name, &new.name) {
        (o, Some(n)) if o.as_ref() != Some(n) => Some((o.clone().unwrap_or_default(), n.clone())),
        _ => None,
    };
    SetChange {
        set_id: new.id.clone(),
        actor: None,
        added,
        removed,
        renamed,
        set_renamed,
        capacity: None,
    }
}

/// The fresh set in every place the old one is held: channels (with their chat lines), the
/// global sets and the people who have it.
fn apply_reload(app: &AppHandle, deck: &str, fresh: &EmoteSet) {
    let state = app.state::<Emotes>();
    let mut notices = Vec::new();
    let mut owners: Vec<String> = Vec::new();
    {
        let mut inner = state.lock();
        let logins: Vec<String> = inner
            .channels
            .iter()
            .filter(|(_, c)| c.sets.iter().any(|s| s.provider == Provider::Cyextv && s.id == deck))
            .map(|(l, _)| l.clone())
            .collect();
        for login in &logins {
            let mut change = None;
            if let Some(c) = inner.channels.get_mut(login) {
                for s in c.sets.iter_mut().filter(|s| s.provider == Provider::Cyextv && s.id == deck) {
                    change = Some(diff(s, fresh));
                    *s = Arc::new(with_scope(fresh, s.scope));
                }
            }
            inner.rebuild(login);
            for (text, emote) in change.iter().flat_map(|c| live::lines_for("cyex.tv", None, c)) {
                notices.push((login.clone(), text, emote));
            }
        }
        if inner.globals.iter().any(|s| s.provider == Provider::Cyextv && s.id == deck) {
            for s in inner.globals.iter_mut().filter(|s| s.provider == Provider::Cyextv && s.id == deck) {
                *s = Arc::new(with_scope(fresh, s.scope));
            }
            inner.rebuild_all();
        }
        if let Some(old) = inner.personal_sets.get(deck).cloned() {
            inner
                .personal_sets
                .insert(deck.to_owned(), Arc::new(with_scope(fresh, old.scope)));
            owners = inner
                .user_sets
                .iter()
                .filter(|(_, sets)| sets.iter().any(|s| s == deck))
                .map(|(u, _)| u.clone())
                .collect();
            for u in &owners {
                inner.rebuild_personal(u);
            }
        }
    }
    for (login, text, emote) in notices {
        super::change_line(app, &login, &text, emote.as_ref());
    }
    if !owners.is_empty() {
        crate::chat::retokenize_users(app, &owners);
    }
}

/// A channel's active set switched: its sets are read again (the topics follow in
/// `retry::merge`), and chat is told when the set really is another.
fn switched(app: &AppHandle, twitch_id: String, to: Option<String>) {
    let state = app.state::<Emotes>();
    let (login, generation, before) = {
        let inner = state.lock();
        if inner.sets_off.cyextv {
            return;
        }
        let Some((login, c)) = inner.channels.iter().find(|(_, c)| c.id == twitch_id) else {
            return;
        };
        let before = c.sets.iter().find(|s| s.provider == Provider::Cyextv).map(|s| s.id.clone());
        (login.clone(), c.generation, before)
    };
    // Already the set shown.
    if to.as_deref().map(api::deck_set_id) == before {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let which = Which {
            cyextv: true,
            ..Which::default()
        };
        let http = app.state::<Auth>().http.clone();
        let lane = crate::chat::shown::lane(&app, &login);
        let got = providers::channel(&http, &twitch_id, lane, which, Which::default()).await;
        if got.failed.cyextv {
            eprintln!("emotes: no cyex.tv set for {login} after its switch");
            return;
        }
        if !retry::merge(&app, &login, generation, which, got) {
            return;
        }
        let after = app
            .state::<Emotes>()
            .lock()
            .channels
            .get(&login)
            .and_then(|c| c.sets.iter().find(|s| s.provider == Provider::Cyextv).cloned());
        if after.as_ref().map(|s| &s.id) != before.as_ref() {
            let name = after.as_ref().and_then(|s| s.name.as_deref());
            let text = live::switch_line_for("cyex.tv", None, name, after.is_none());
            super::change_line(&app, &login, &text, None);
        }
    });
}

/// The socket was down for good, or the server says everything may have changed: the global set,
/// each open channel's and the personal sets held are read again.
fn resync(app: &AppHandle) {
    if !crate::hermes::resync_due("cyextv") {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let which = Which {
            cyextv: true,
            ..Which::default()
        };
        let (joined, users) = {
            let state = app.state::<Emotes>();
            let mut inner = state.lock();
            if inner.sets_off.cyextv {
                return;
            }
            inner.globals_done = inner.globals_done.minus(which);
            let joined: Vec<(String, String, u64)> = inner
                .channels
                .iter()
                .map(|(l, c)| (l.clone(), c.id.clone(), c.generation))
                .collect();
            let users: Vec<String> = inner
                .user_sets
                .iter()
                .filter(|(_, sets)| sets.iter().any(|s| s.starts_with("cyextv-")))
                .map(|(u, _)| u.clone())
                .collect();
            (joined, users)
        };
        load::globals(&app);
        let http = app.state::<Auth>().http.clone();
        for (login, id, generation) in joined {
            let lane = crate::chat::shown::lane(&app, &login);
            let got = providers::channel(&http, &id, lane, which, Which::default()).await;
            retry::merge(&app, &login, generation, which, got);
        }
        super::cyextv::again(&app, &users);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emote(id: &str, name: &str) -> Emote {
        Emote::new(Provider::Cyextv, id, name)
    }

    fn set(emotes: Vec<Emote>, name: Option<&str>) -> EmoteSet {
        let mut s = EmoteSet::new("cyextv-S", Provider::Cyextv, Scope::Channel, emotes);
        s.name = name.map(str::to_owned);
        s
    }

    fn texts(c: &SetChange) -> Vec<String> {
        live::lines_for("cyex.tv", None, c).into_iter().map(|(t, _)| t).collect()
    }

    #[test]
    fn v112_06_a_new_read_tells_what_changed() {
        let old = set(vec![emote("1.k", "a"), emote("2.k", "b"), emote("3.k", "c")], Some("Main"));
        let new = set(vec![emote("2.k", "b"), emote("3.k", "see"), emote("4.k", "d")], Some("Main"));
        let c = diff(&old, &new);
        assert_eq!(c.added.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["d"]);
        assert_eq!(c.removed, [("1.k".to_owned(), "a".to_owned())]);
        assert_eq!(c.renamed.len(), 1);
        assert_eq!((c.renamed[0].1.as_str(), c.renamed[0].2.name.as_str()), ("c", "see"));
        assert_eq!(c.set_renamed, None);
        assert_eq!(
            texts(&c),
            [
                "The cyex.tv emote d was added.",
                "The cyex.tv emote a was removed.",
                "The cyex.tv emote c is now see."
            ]
        );
    }

    #[test]
    fn v112_06_nothing_changed_says_nothing() {
        let s = set(vec![emote("1.k", "a")], Some("Main"));
        let c = diff(&s, &s);
        assert!(texts(&c).is_empty());
        // A set without a name stays so; one named for the first time is a rename.
        let unnamed = set(vec![], None);
        assert_eq!(diff(&unnamed, &unnamed).set_renamed, None);
        assert_eq!(
            diff(&unnamed, &set(vec![], Some("New"))).set_renamed,
            Some((String::new(), "New".to_owned()))
        );
        assert_eq!(texts(&diff(&set(vec![], Some("Old")), &set(vec![], Some("New")))), ["The cyex.tv set Old is now New."]);
    }

    #[test]
    fn v112_06_two_names_for_one_emote_are_told_apart() {
        let old = set(vec![emote("1.k", "a1"), emote("1.k", "a2")], None);
        let new = set(vec![emote("1.k", "a1")], None);
        let c = diff(&old, &new);
        assert_eq!(c.removed, [("1.k".to_owned(), "a2".to_owned())]);
        assert!(c.added.is_empty() && c.renamed.is_empty());
    }

    #[test]
    fn v112_06_a_set_keeps_the_scope_it_is_held_in() {
        let fresh = set(vec![emote("1.k", "a")], Some("Main"));
        assert_eq!(with_scope(&fresh, Scope::Global).scope, Scope::Global);
        assert_eq!(with_scope(&fresh, Scope::Personal).emotes.len(), 1);
    }
}
