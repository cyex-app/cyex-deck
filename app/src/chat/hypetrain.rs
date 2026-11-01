//! V108-N04 The hype train bar over a chat: the train read from GQL, kept current by the
//! PubSub topic `hype-train-events-v2.<channel id>`, and a moment in the chat's timeline when
//! one starts or levels up. The payloads are read from twitch.tv's bundle, not seen live, so
//! a message that doesn't parse only tells the UI to read the train again.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use cyex_twitch::gql::Auth as GqlAuth;
use cyex_twitch::gql::hypetrain::{self as api, Conductor, Event as Wire, Train};
use cyex_twitch::hermes::{Event, Kind};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use ts_rs::TS;

use super::{Chat, now_ms};
use crate::auth::Auth;
use cyex_chat::moments::MomentKind;

const EVENT: &str = "chat:hype-train";

/// The train id and level last seen per channel id, so a rise is a moment and a repeat isn't.
static LEVELS: LazyLock<Mutex<HashMap<String, Seen>>> = LazyLock::new(Mutex::default);

/// V108-N04 A read is kept this long per login and kind, so the chats of one channel (split,
/// popped out, all told to read again by one topic message) ask Twitch once.
const READ_AGE: Duration = Duration::from_secs(10);

/// (login, ended) for [`api::current`] or [`api::latest`].
type ReadKey = (String, bool);

struct Kept {
    at: Instant,
    channel_id: Option<String>,
    train: Option<Train>,
}

/// Reads kept a moment. Any topic message drops its channel's, and a read asked for before
/// one isn't kept (`generation`).
#[derive(Default)]
struct Reads {
    kept: HashMap<ReadKey, Kept>,
    generation: u64,
}

impl Reads {
    fn get(&self, key: &ReadKey, now: Instant) -> Option<Option<Train>> {
        self.kept.get(key).filter(|k| now.duration_since(k.at) < READ_AGE).map(|k| k.train.clone())
    }

    fn put(&mut self, key: ReadKey, kept: Kept, generation: u64) {
        if generation != self.generation {
            return;
        }
        let now = kept.at;
        self.kept.retain(|_, k| now.duration_since(k.at) < READ_AGE);
        self.kept.insert(key, kept);
    }

    fn forget(&mut self, channel_id: &str, login: Option<&str>) {
        self.generation += 1;
        self.kept.retain(|(l, _), k| k.channel_id.as_deref() != Some(channel_id) && Some(l.as_str()) != login);
    }
}

static READS: LazyLock<Mutex<Reads>> = LazyLock::new(Mutex::default);
/// One read in flight per key; the others wait for it and take the kept answer.
static GATES: LazyLock<Mutex<HashMap<ReadKey, Arc<tokio::sync::Mutex<()>>>>> = LazyLock::new(Mutex::default);

fn reads() -> std::sync::MutexGuard<'static, Reads> {
    READS.lock().unwrap_or_else(|p| p.into_inner())
}

async fn kept_or<F: Future<Output = Result<Option<Train>, String>>>(
    app: &AppHandle,
    login: &str,
    ended: bool,
    fetch: F,
) -> Result<Option<Train>, String> {
    let key = (login.to_owned(), ended);
    let gate = GATES.lock().unwrap_or_else(|p| p.into_inner()).entry(key.clone()).or_default().clone();
    let _one = gate.lock().await;
    let generation = {
        let r = reads();
        if let Some(t) = r.get(&key, Instant::now()) {
            return Ok(t);
        }
        r.generation
    };
    let train = fetch.await?;
    let kept = Kept { at: Instant::now(), channel_id: super::channel_id_of(app, login), train: train.clone() };
    reads().put(key, kept, generation);
    Ok(train)
}

#[derive(Clone, Debug, PartialEq)]
struct Seen {
    train: String,
    level: u32,
}

/// What changed in a channel's train.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum HypeChange {
    /// Progress or a level-up; the train has no conductors or start time here.
    Progress { train: Train },
    Conductor { conductor: Conductor },
    End,
    /// Something the app couldn't read: read the train again.
    Reread,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HypeChanged {
    pub channel_id: String,
    pub change: HypeChange,
}

fn now_text() -> String {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .ok()
        .and_then(|t| t.format(&Rfc3339).ok())
        .unwrap_or_default()
}

/// The channel's train now, `None` when there is none.
pub async fn read(app: &AppHandle, login: &str) -> Result<Option<Train>, String> {
    let http = app.state::<Auth>().http.clone();
    let login = login.to_ascii_lowercase();
    let fetch = async { api::current(&http, GqlAuth::PUBLIC, &login, &now_text()).await.map_err(told) };
    let train = kept_or(app, &login, false, fetch).await?;
    let login = login.as_str();
    if let Some(id) = super::channel_id_of(app, login) {
        let mut levels = LEVELS.lock().unwrap_or_else(|p| p.into_inner());
        match &train {
            // Seed only: a poll must not swallow the level-up the topic is about to report.
            Some(t) => {
                let seen = Seen { train: t.id.clone(), level: t.level };
                let entry = levels.entry(id).or_insert_with(|| seen.clone());
                if !same_train(&entry.train, &t.id) {
                    *entry = seen;
                }
            }
            // V108-N04 No train: a stale level would hide the next train's moments.
            None => {
                levels.remove(&id);
            }
        }
    }
    Ok(train)
}

/// The channel's last train, ended or not, for the closing stats. Leaves the seen levels.
pub async fn last(app: &AppHandle, login: &str) -> Result<Option<Train>, String> {
    let http = app.state::<Auth>().http.clone();
    let login = login.to_ascii_lowercase();
    let fetch = async { api::latest(&http, GqlAuth::PUBLIC, &login).await.map_err(told) };
    kept_or(app, &login, true, fetch).await
}

fn told(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        "Couldn’t reach Twitch.".to_owned()
    } else {
        eprintln!("hype train: {e}");
        "Twitch didn’t answer. Try again in a bit.".to_owned()
    }
}

/// Follows (or stops following) the topic while a chat of a live channel shows the bar. The
/// topic is counted, so several chats can share it. Held per window (`label`), so a page that
/// goes away without its cleanup lets go too (`hermes::release`).
pub fn watch(app: &AppHandle, label: &str, channel_id: &str, on: bool) {
    if channel_id.is_empty() {
        return;
    }
    let held = crate::hermes::Held::Channel(Kind::HypeTrain, channel_id.to_owned());
    if on {
        crate::hermes::hold(app, label, held, true);
    } else if crate::hermes::forget(label, &held) {
        unwatched(app, channel_id);
    }
}

/// One count of the topic let go. Other chats of the channel may still follow it; the level
/// goes with the last one.
pub fn unwatched(app: &AppHandle, channel_id: &str) {
    crate::hermes::remove(app, Kind::HypeTrain, channel_id);
    if !crate::hermes::held(app, Kind::HypeTrain, channel_id) {
        LEVELS.lock().unwrap_or_else(|p| p.into_inner()).remove(channel_id);
    }
}

/// A message of the topic.
pub fn on_event(app: &AppHandle, e: &Event) {
    let id = e.topic.channel_id.clone();
    let login = app.state::<Chat>().channels.login_of(&id);
    reads().forget(&id, login.as_deref());
    let change = match api::event_of(&e.payload) {
        Some(Wire::Progress { train, .. }) => {
            rose(app, &id, &train);
            HypeChange::Progress { train }
        }
        Some(Wire::Conductor(conductor)) => HypeChange::Conductor { conductor },
        Some(Wire::End) => {
            LEVELS.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
            HypeChange::End
        }
        // The approaching notice and the rewards aren't shown.
        Some(Wire::Approaching { .. }) => return,
        None if e.payload["type"].is_string() => return,
        None => HypeChange::Reread,
    };
    let _ = app.emit(EVENT, HypeChanged { channel_id: id, change });
}

/// A topic message may carry no id; it then belongs to the train already seen.
fn same_train(seen: &str, id: &str) -> bool {
    id.is_empty() || seen.is_empty() || seen == id
}

/// The level last seen of this train; another train's level doesn't count (V108-N04).
fn last_level(seen: Option<&Seen>, train: &str) -> Option<u32> {
    seen.filter(|s| same_train(&s.train, train)).map(|s| s.level)
}

/// The label of a moment when `level` is higher than the last seen one.
fn moment_label(last: Option<u32>, level: u32) -> Option<String> {
    match last {
        None if level <= 1 => Some("Hype train started".to_owned()),
        None => Some(format!("Hype train, level {level}")),
        Some(l) if level > l => Some(format!("Hype train, level {level}")),
        _ => None,
    }
}

fn rose(app: &AppHandle, channel_id: &str, train: &Train) {
    let label = {
        let mut levels = LEVELS.lock().unwrap_or_else(|p| p.into_inner());
        let label = moment_label(last_level(levels.get(channel_id), &train.id), train.level);
        if label.is_some() {
            levels.insert(channel_id.to_owned(), Seen { train: train.id.clone(), level: train.level });
        }
        label
    };
    let Some(label) = label else { return };
    let Some(login) = app.state::<Chat>().channels.login_of(channel_id) else { return };
    let now = now_ms();
    let mut found = Vec::new();
    {
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        for ((_, l), entry) in inner.rooms.iter_mut() {
            if *l != login {
                continue;
            }
            entry.room.moment(MomentKind::Hype, &label, now);
            let fresh = entry.room.take_moments();
            if found.is_empty() {
                found = fresh;
            }
        }
    }
    if !found.is_empty() {
        super::moments::announce(app, &login, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(channel: &str, at: Instant) -> Kept {
        Kept { at, channel_id: Some(channel.to_owned()), train: None }
    }

    #[test]
    fn reads_are_kept_a_moment_and_dropped_by_the_topic() {
        let mut r = Reads::default();
        let t = Instant::now();
        let key = |l: &str, ended| (l.to_owned(), ended);
        r.put(key("a", false), kept("1", t), 0);
        r.put(key("a", true), kept("1", t), 0);
        r.put(key("b", false), kept("2", t), 0);
        assert_eq!(r.get(&key("a", false), t + Duration::from_secs(9)), Some(None));
        assert_eq!(r.get(&key("a", false), t + READ_AGE), None);
        // A message for channel 1 drops both of its kinds, not channel 2's.
        r.forget("1", None);
        assert_eq!(r.get(&key("a", false), t), None);
        assert_eq!(r.get(&key("a", true), t), None);
        assert_eq!(r.get(&key("b", false), t), Some(None));
        // By login too, when the read didn't know the id.
        r.put(key("c", false), Kept { at: t, channel_id: None, train: None }, r.generation);
        r.forget("3", Some("c"));
        assert_eq!(r.get(&key("c", false), t), None);
    }

    #[test]
    fn a_read_from_before_a_message_is_not_kept() {
        let mut r = Reads::default();
        let t = Instant::now();
        let asked = r.generation;
        r.forget("1", None);
        r.put(("a".to_owned(), false), kept("1", t), asked);
        assert_eq!(r.get(&("a".to_owned(), false), t), None);
    }

    #[test]
    fn moments_on_start_and_level_ups_only() {
        assert_eq!(moment_label(None, 1).as_deref(), Some("Hype train started"));
        assert_eq!(moment_label(None, 3).as_deref(), Some("Hype train, level 3"));
        assert_eq!(moment_label(Some(2), 3).as_deref(), Some("Hype train, level 3"));
        assert_eq!(moment_label(Some(3), 3), None);
        assert_eq!(moment_label(Some(3), 2), None);
    }

    #[test]
    fn another_train_starts_over() {
        let seen = Seen { train: "a".to_owned(), level: 4 };
        assert_eq!(last_level(Some(&seen), "a"), Some(4));
        assert_eq!(last_level(Some(&seen), ""), Some(4));
        assert_eq!(last_level(Some(&seen), "b"), None);
        assert_eq!(moment_label(last_level(Some(&seen), "b"), 1).as_deref(), Some("Hype train started"));
        assert_eq!(last_level(None, "b"), None);
    }
}
