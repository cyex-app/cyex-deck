//! V107-R8 While the miner is off, its view still shows avatars and who's live: one
//! batched lookup when the view opens, at most once a minute. Nothing polls.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cyex_twitch::gql::miner::{self as gm, MinerChannel};
use tauri::AppHandle;

use super::{MinedChannel, http, lock, settings};

const AGE: Duration = Duration::from_secs(60);

struct Cache {
    at: Instant,
    list: Vec<MinedChannel>,
    /// Ids already looked up: a channel added since is looked up alone, cache or not.
    asked: HashSet<String>,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

/// The channels the view lists: first, your list and skipped ones.
fn ids(s: &cyex_settings::schema::Miner) -> Vec<String> {
    let mut seen = HashSet::new();
    s.first
        .iter()
        .chain(&s.channels)
        .chain(&s.skip)
        .filter(|c| seen.insert(c.id.clone()))
        .map(|c| c.id.clone())
        .collect()
}

fn mined(c: MinerChannel) -> MinedChannel {
    MinedChannel {
        live: c.stream.is_some(),
        game: c.stream.and_then(|s| s.game.map(|g| g.1)),
        id: c.id,
        login: c.login,
        name: c.name,
        avatar: c.avatar,
        mining: false,
        watching: false,
        first: false,
        balance: None,
        streak: false,
        drops: false,
    }
}

/// Fresh while younger than `AGE`; a lookup in flight counts as fresh.
fn fresh(c: &Option<Cache>, now: Instant) -> bool {
    c.as_ref().is_some_and(|c| now.duration_since(c.at) < AGE)
}

/// V107-R8b Ids the cache hasn't looked up yet.
fn unasked(c: &Option<Cache>, ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|i| c.as_ref().is_none_or(|c| !c.asked.contains(*i)))
        .cloned()
        .collect()
}

/// V107-R8b Looks up only the new channels and adds them to the cached list.
async fn peek_new(app: &AppHandle, new: Vec<String>) -> Vec<MinedChannel> {
    let found = gm::streams(&http(app), &new).await;
    let mut c = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    let Some(c) = c.as_mut() else {
        return Vec::new();
    };
    match found {
        Ok(found) => {
            c.asked.extend(new);
            c.list.retain(|m| !found.iter().any(|f| f.id == m.id));
            c.list.extend(found.into_iter().map(mined));
        }
        Err(e) => eprintln!("miner: peek failed: {e}"),
    }
    c.list.clone()
}

enum Start {
    Cached(Vec<MinedChannel>),
    New(Vec<String>),
    All,
}

/// Decides, under the lock, what the lookup has to cover.
fn begin(ids: &[String]) -> Start {
    let mut c = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    if fresh(&c, Instant::now()) {
        let new = unasked(&c, ids);
        if new.is_empty() {
            return Start::Cached(c.as_ref().map(|c| c.list.clone()).unwrap_or_default());
        }
        return Start::New(new);
    }
    let list = c.take().map(|c| c.list).unwrap_or_default();
    *c = Some(Cache {
        at: Instant::now(),
        list,
        asked: HashSet::new(),
    });
    Start::All
}

pub async fn peek(app: &AppHandle) -> Vec<MinedChannel> {
    if lock(app).account.is_some() {
        // Running: the status has all of it.
        return Vec::new();
    }
    let ids = ids(&settings(app));
    match begin(&ids) {
        Start::Cached(list) => return list,
        Start::New(new) => return peek_new(app, new).await,
        Start::All => {}
    }
    if ids.is_empty() {
        return Vec::new();
    }
    match gm::streams(&http(app), &ids).await {
        Ok(found) => {
            let list: Vec<MinedChannel> = found.into_iter().map(mined).collect();
            let mut c = CACHE.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(c) = c.as_mut() {
                c.list = list.clone();
                c.asked = ids.iter().cloned().collect();
            }
            list
        }
        Err(e) => {
            eprintln!("miner: peek failed: {e}");
            // Next open tries again.
            let mut c = CACHE.lock().unwrap_or_else(|p| p.into_inner());
            let list = c.take().map(|c| c.list).unwrap_or_default();
            *c = Some(Cache {
                at: Instant::now().checked_sub(AGE).unwrap_or_else(Instant::now),
                list: list.clone(),
                asked: HashSet::new(),
            });
            list
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_settings::schema::{Miner as MinerSettings, TabChannel};

    fn ch(id: &str) -> TabChannel {
        TabChannel {
            id: id.into(),
            login: id.into(),
            name: id.into(),
        }
    }

    #[test]
    fn every_listed_channel_once() {
        let s = MinerSettings {
            first: vec![ch("1")],
            channels: vec![ch("1"), ch("2")],
            skip: vec![ch("3"), ch("2")],
            ..MinerSettings::default()
        };
        assert_eq!(ids(&s), ["1", "2", "3"]);
    }

    #[test]
    fn new_channels_skip_the_cache() {
        let ids = vec!["1".to_string(), "2".to_string()];
        assert_eq!(unasked(&None, &ids), ["1", "2"]);
        let c = Some(Cache {
            at: Instant::now(),
            list: Vec::new(),
            asked: HashSet::from(["1".to_string()]),
        });
        assert_eq!(unasked(&c, &ids), ["2"]);
        let all = vec!["1".to_string()];
        assert!(unasked(&c, &all).is_empty());
    }

    #[test]
    fn once_a_minute() {
        let now = Instant::now();
        assert!(!fresh(&None, now));
        let c = |ago: u64| {
            Some(Cache {
                at: now - Duration::from_secs(ago),
                list: Vec::new(),
                asked: HashSet::new(),
            })
        };
        assert!(fresh(&c(10), now));
        assert!(!fresh(&c(60), now));
    }
}
