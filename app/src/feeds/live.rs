//! V106-07 Live notifications feed (Q5): streams of tab channels and followed channels
//! starting or ending, from the tab check (`tabs::live`) and the follow check (`alerts`).

use std::collections::HashMap;
use std::sync::Mutex;

use cyex_settings::schema::FeedKind;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{FeedEntry, push};

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LiveEntry {
    pub key: String,
    #[ts(type = "number")]
    pub ts: i64,
    pub channel_id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
    pub live: bool,
    pub title: Option<String>,
    pub game: Option<String>,
}

/// A stream that started or ended, as a check saw it.
pub struct Flip {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
    pub live: bool,
    pub title: Option<String>,
    pub game: Option<String>,
    /// The stream's start time; a new one means a restart.
    pub started: Option<String>,
}

/// Channel id → (live, stream start), as last put in the feed.
#[derive(Default)]
pub struct Last(pub Mutex<HashMap<String, (bool, Option<String>)>>);

/// A stream started or ended. Both checks (tabs, follows) may see the same flip; the last state per channel decides.
pub fn flip(app: &AppHandle, f: Flip) {
    let Some(last) = app.try_state::<Last>() else {
        return;
    };
    if !is_new(
        &mut last.0.lock().unwrap_or_else(|p| p.into_inner()),
        &f.id,
        f.live,
        f.started.clone(),
    ) {
        return;
    }
    let ts = crate::chat::now_ms();
    let entry = LiveEntry {
        key: format!("{}:{ts}", f.id),
        ts,
        channel_id: f.id,
        login: f.login,
        name: f.name,
        avatar: f.avatar,
        live: f.live,
        title: f.title,
        game: f.game,
    };
    push(app, FeedKind::Live, FeedEntry::Live(entry), |_| false);
}

/// True (and stores it) when the state differs from the last one, or none is known.
/// Two known start times that differ are a restart.
fn is_new(last: &mut HashMap<String, (bool, Option<String>)>, id: &str, live: bool, started: Option<String>) -> bool {
    let old = last.get(id).cloned();
    let kept = started.clone().or_else(|| old.as_ref().and_then(|o| o.1.clone()));
    let new = match &old {
        None => true,
        Some((l, s)) => *l != live || matches!((&started, s), (Some(a), Some(b)) if a != b),
    };
    last.insert(id.to_owned(), (live, kept));
    new
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_flip_per_change() {
        let mut last = HashMap::new();
        // The tab check and the follow check both see one stream start.
        let s = |t: &str| Some(t.to_owned());
        assert!(is_new(&mut last, "1", true, s("a")));
        assert!(!is_new(&mut last, "1", true, s("a")));
        assert!(!is_new(&mut last, "1", true, None));
        // A restart: same live state, new start time.
        assert!(is_new(&mut last, "1", true, s("b")));
        // Live, offline, live again: three.
        let mut n = 0;
        for live in [false, true] {
            n += usize::from(is_new(&mut last, "1", live, None));
        }
        assert_eq!(n, 2);
    }
}
