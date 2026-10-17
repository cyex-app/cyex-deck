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
}

/// Channel id → live, as last put in the feed.
#[derive(Default)]
pub struct Last(pub Mutex<HashMap<String, bool>>);

/// A stream started or ended. Both checks (tabs, follows) may see the same flip; the last state per channel decides.
pub fn flip(app: &AppHandle, f: Flip) {
    let Some(last) = app.try_state::<Last>() else {
        return;
    };
    if !is_new(
        &mut last.0.lock().unwrap_or_else(|p| p.into_inner()),
        &f.id,
        f.live,
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
fn is_new(last: &mut HashMap<String, bool>, id: &str, live: bool) -> bool {
    last.insert(id.to_owned(), live) != Some(live)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_flip_per_change() {
        let mut last = HashMap::new();
        // The tab check and the follow check both see one stream start.
        assert!(is_new(&mut last, "1", true));
        assert!(!is_new(&mut last, "1", true));
        // Live, offline, live again: three.
        let mut n = 1;
        for live in [false, true] {
            n += usize::from(is_new(&mut last, "1", live));
        }
        assert_eq!(n, 3);
    }
}
