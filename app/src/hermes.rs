//! One place for Hermes (D14): anonymous connections for channel topics, shared by chat
//! (pins, redemptions, polls, predictions, celebrations) and the miner, and one signed-in
//! connection per account for its own topics (points, bets, whispers, the bell). Topics are counted, so two
//! features can follow the same one. Events go to whoever handles their kind.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use cyex_twitch::hermes::{Event, Hermes, Kind, Topic};
use cyex_twitch::ws::Pool;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

pub struct Hub {
    anon: Pool<Hermes>,
    /// By account id.
    users: Mutex<HashMap<String, Pool<Hermes>>>,
    events: mpsc::UnboundedSender<Event>,
}

pub fn init(app: &AppHandle) {
    let (tx, rx) = mpsc::unbounded_channel();
    app.manage(Hub {
        anon: Pool::new(
            {
                let app = app.clone();
                move || Hermes::anonymous().on_resync(resync(&app, None))
            },
            tx.clone(),
            spawn,
        ),
        users: Mutex::default(),
        events: tx,
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move { pump(app, rx).await });
}

fn spawn(task: Pin<Box<dyn Future<Output = ()> + Send>>) {
    tauri::async_runtime::spawn(task);
}

/// Follows a channel topic.
pub fn add(app: &AppHandle, kind: Kind, channel_id: &str) {
    if let Some(hub) = app.try_state::<Hub>() {
        hub.anon.add(Topic::new(kind, channel_id));
    }
}

pub fn remove(app: &AppHandle, kind: Kind, channel_id: &str) {
    if let Some(hub) = app.try_state::<Hub>() {
        hub.anon.remove(&Topic::new(kind, channel_id));
    }
}

/// Whether anything still follows the channel topic (the topic is counted).
pub fn held(app: &AppHandle, kind: Kind, channel_id: &str) -> bool {
    app.try_state::<Hub>().is_some_and(|hub| hub.anon.has(&Topic::new(kind, channel_id)))
}

/// Follows one of an account's own topics, signed in with its GQL token.
pub fn add_user(app: &AppHandle, account: &str, kind: Kind) {
    add_user_topic(app, account, kind, account);
}

/// Like `add_user`, for a topic with its own id, e.g. V103-15's `<user>.<channel>`.
pub fn add_user_topic(app: &AppHandle, account: &str, kind: Kind, id: &str) {
    let Some(hub) = app.try_state::<Hub>() else { return };
    let mut users = hub.users.lock().unwrap_or_else(|p| p.into_inner());
    let pool = users.entry(account.to_owned()).or_insert_with(|| {
        let (app, id) = (app.clone(), account.to_owned());
        let make = move || {
            let (app, id) = (app.clone(), id.clone());
            let on_resync = resync(&app, Some(id.clone()));
            Hermes::signed_in(Box::new(move || {
                crate::auth::gql_token(&app, &id).map(|(token, _)| token)
            }))
            .on_resync(on_resync)
        };
        Pool::new(make, hub.events.clone(), spawn)
    });
    pool.add(Topic::new(kind, id));
}

pub fn remove_user(app: &AppHandle, account: &str, kind: Kind) {
    remove_user_topic(app, account, kind, account);
}

pub fn remove_user_topic(app: &AppHandle, account: &str, kind: Kind, id: &str) {
    let Some(hub) = app.try_state::<Hub>() else { return };
    let users = hub.users.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(pool) = users.get(account) {
        pool.remove(&Topic::new(kind, id));
    }
}

/// Least time between two reads after reconnects for one connection owner.
const RESYNC_GAP: Duration = Duration::from_secs(30);

/// When each owner last read after a reconnect; a socket that flaps doesn't hammer the APIs.
static RESYNCED: LazyLock<Mutex<HashMap<String, Instant>>> = LazyLock::new(Mutex::default);

/// Whether `key` may read again now (and notes that it did). `crate::emotes` uses it too.
pub(crate) fn resync_due(key: &str) -> bool {
    due(&mut RESYNCED.lock().unwrap_or_else(|p| p.into_inner()), key, Instant::now())
}

fn due(last: &mut HashMap<String, Instant>, key: &str, now: Instant) -> bool {
    if last.get(key).is_some_and(|t| now.saturating_duration_since(*t) < RESYNC_GAP) {
        return false;
    }
    last.insert(key.to_owned(), now);
    true
}

/// Called when a connection is back after a drop: what happened meanwhile is read again.
/// Anonymous (`None`): polls and predictions of the open chats. An account: its whispers,
/// bell count and the miner's channels.
fn resync(app: &AppHandle, account: Option<String>) -> cyex_twitch::hermes::Resync {
    let app = app.clone();
    Arc::new(move || {
        let key = account.as_deref().unwrap_or("");
        if !resync_due(&format!("hermes:{key}")) {
            return;
        }
        match account.as_deref() {
            None => crate::chat::votes::resync(&app),
            Some(id) => {
                crate::whispers::resync(&app, id);
                crate::bell::resync(&app, id);
                crate::miner::resync(&app, id);
            }
        }
    })
}

async fn pump(app: AppHandle, mut rx: mpsc::UnboundedReceiver<Event>) {
    while let Some(e) = rx.recv().await {
        match e.topic.kind {
            Kind::Polls | Kind::Celebrations => crate::chat::votes::on_event(&app, &e),
            Kind::Predictions => {
                crate::chat::votes::on_event(&app, &e);
                crate::miner::on_channel_event(&app, &e);
            }
            Kind::Pins => crate::chat::live::on_event(&app, &e),
            Kind::Points => {
                crate::chat::live::on_event(&app, &e);
                crate::miner::on_channel_event(&app, &e);
            }
            Kind::Playback | Kind::Raid => {
                crate::miner::on_channel_event(&app, &e);
                crate::streammanager::on_hermes(&app, &e);
            }
            Kind::Moments => crate::miner::on_channel_event(&app, &e),
            Kind::UserPoints => {
                crate::miner::on_user_event(&app, &e);
                // V108-N23 A WATCH_STREAK gain: the streak prompt reads again.
                crate::subs::on_points(&app, &e);
            }
            Kind::UserPredictions => crate::miner::on_user_event(&app, &e),
            Kind::Whispers => crate::whispers::on_hermes(&app, &e),
            Kind::Onsite => crate::bell::on_event(&app, &e),
            Kind::ActivityAlerts
            | Kind::ActivityFeed
            | Kind::BroadcastSettings
            | Kind::AdsManager
            | Kind::GuestStarUser => crate::streammanager::on_hermes(&app, &e),
            Kind::ModActions | Kind::AutomodQueue => crate::moderation::feeds::on_hermes(&app, &e),
            // V108-N23 The streak prompt reads again.
            Kind::ViewerMilestones => crate::subs::on_milestones(&app, &e),
            // V108-N04 The hype train bar reads and merges it.
            Kind::HypeTrain => crate::chat::hypetrain::on_event(&app, &e),
        }
    }
}

/// V108-N04/N23 A topic a window's page follows, so it's let go when the page goes away
/// without its own cleanup (a popout closed, a reload).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Held {
    Channel(Kind, String),
    User(String, Kind),
}

/// What each window's page follows, by window label; one entry per `on`.
static BY_WINDOW: LazyLock<Mutex<HashMap<String, Vec<Held>>>> = LazyLock::new(Mutex::default);

/// Notes one `on` or `off` of `label`. False for an `off` the window never asked for (it was
/// released already), which then must not let go of anyone else's count.
fn note(by: &mut HashMap<String, Vec<Held>>, label: &str, held: Held, on: bool) -> bool {
    if on {
        by.entry(label.to_owned()).or_default().push(held);
        return true;
    }
    let Some(list) = by.get_mut(label) else { return false };
    let Some(i) = list.iter().position(|h| *h == held) else { return false };
    list.swap_remove(i);
    if list.is_empty() {
        by.remove(label);
    }
    true
}

/// Follows (or stops following) a topic for a window's page. False when nothing changed.
pub fn hold(app: &AppHandle, label: &str, held: Held, on: bool) -> bool {
    if !note(&mut BY_WINDOW.lock().unwrap_or_else(|p| p.into_inner()), label, held.clone(), on) {
        return false;
    }
    match (held, on) {
        (Held::Channel(kind, id), true) => add(app, kind, &id),
        (Held::Channel(kind, id), false) => remove(app, kind, &id),
        (Held::User(account, kind), true) => add_user(app, &account, kind),
        (Held::User(account, kind), false) => remove_user(app, &account, kind),
    }
    true
}

/// Drops one of the window's notes without letting go of the topic, for a caller that does
/// more than `remove` (the hype train's level). False when the window didn't hold it.
pub fn forget(label: &str, held: &Held) -> bool {
    note(&mut BY_WINDOW.lock().unwrap_or_else(|p| p.into_inner()), label, held.clone(), false)
}

/// Lets go of everything a window's page followed: it closed or loads again.
pub fn release(app: &AppHandle, label: &str) {
    let gone = BY_WINDOW.lock().unwrap_or_else(|p| p.into_inner()).remove(label).unwrap_or_default();
    for held in gone {
        match held {
            Held::Channel(Kind::HypeTrain, id) => crate::chat::hypetrain::unwatched(app, &id),
            Held::Channel(kind, id) => remove(app, kind, &id),
            Held::User(account, kind) => remove_user(app, &account, kind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resyncs_wait_out_the_gap() {
        let mut last = HashMap::new();
        let t = Instant::now();
        assert!(due(&mut last, "a", t));
        assert!(!due(&mut last, "a", t + Duration::from_secs(5)));
        assert!(due(&mut last, "b", t + Duration::from_secs(5)));
        assert!(due(&mut last, "a", t + RESYNC_GAP));
    }

    #[test]
    fn released_windows_dont_let_go_twice() {
        let mut by = HashMap::new();
        let h = || Held::Channel(Kind::HypeTrain, "1".to_owned());
        assert!(note(&mut by, "a", h(), true));
        assert!(note(&mut by, "a", h(), true));
        assert!(note(&mut by, "a", h(), false));
        assert_eq!(by["a"].len(), 1);
        // Another window's off never takes this one's count.
        assert!(!note(&mut by, "b", h(), false));
        by.remove("a");
        assert!(!note(&mut by, "a", h(), false));
        assert!(by.is_empty());
    }
}
