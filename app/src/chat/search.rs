//! F1 Chat search: one chat, or every joined chat (C29). Background rooms keep only
//! `cyex_chat::BACKGROUND` lines (V107-M1), so for ones that dropped lines the search also
//! reads the history service, best effort, up to the scrollback. Those lines come back as
//! history, like any other loaded from the service. Fetched lines are kept briefly, so
//! typing a query doesn't fetch again on every key.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cyex_chat::Room;
use cyex_chat::model::ChatMessage;
use cyex_chat::search::Query;
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};

use super::feeds::FeedLine;
use super::{Chat, now_ms};

/// Most results.
const MAX: usize = 300;
/// How long fetched history serves later queries.
const KEEP: Duration = Duration::from_secs(30);
/// A slow history service doesn't hold the search up longer.
const WAIT: Duration = Duration::from_secs(3);
/// The history service keeps no more.
const SERVICE_MAX: usize = 800;

type Lines = Arc<Vec<Arc<ChatMessage>>>;

/// History fetched for searches, by login.
#[derive(Default)]
struct Fetched(Mutex<HashMap<String, (Instant, Lines)>>);

pub fn init(app: &AppHandle) {
    app.manage(Fetched::default());
}

/// Whether a search should also read the history service for this room: it's in the
/// background and dropped lines.
fn cut(room: &Room) -> bool {
    room.background() && room.trimmed()
}

/// Messages matching a search, oldest first. `view` searches its own chat; `None`
/// searches every joined chat.
pub async fn search(app: &AppHandle, view: Option<u32>, query: &str) -> Result<Vec<FeedLine>, String> {
    let q = Query::parse(query);
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let (mut out, mut back, scrollback) = {
        let chat = app.state::<Chat>();
        let inner = chat.lock();
        let only = match view {
            Some(v) => Some(inner.views.get(&v).map(|v| v.0.clone()).ok_or("This chat is closed.")?),
            None => None,
        };
        let mut out = Vec::new();
        let mut back = Vec::new();
        for (key, entry) in &inner.rooms {
            if only.as_ref().is_some_and(|o| o != key) {
                continue;
            }
            if cut(&entry.room) {
                back.push(key.1.clone());
            }
            for m in entry.room.snapshot().messages {
                if q.matches(&m, &key.1) {
                    out.push(FeedLine { channel: key.1.clone(), message: m, global: false });
                }
            }
        }
        (out, back, chat.scrollback.load(Ordering::Relaxed))
    };
    back.sort_unstable();
    back.dedup();
    if !back.is_empty() && app.state::<crate::settings::Settings>().read::<ChatSettings>(Partition::Chat).history {
        for (login, lines) in older(app, back, scrollback.min(SERVICE_MAX)).await {
            for m in lines.iter().filter(|m| q.matches(m, &login)) {
                out.push(FeedLine { channel: login.clone(), message: m.clone(), global: false });
            }
        }
    }
    Ok(finish(out))
}

/// Each line once (the kept copy first), oldest first, the newest `MAX`.
fn finish(out: Vec<FeedLine>) -> Vec<FeedLine> {
    let mut seen = HashSet::new();
    let mut out: Vec<FeedLine> = out.into_iter().filter(|l| seen.insert(l.message.id.clone())).collect();
    out.sort_by_key(|l| l.message.ts);
    let skip = out.len().saturating_sub(MAX);
    out.drain(..skip);
    out
}

/// History service lines of these channels, from the last `KEEP` or fetched now. A
/// channel that fails or takes too long is left out.
async fn older(app: &AppHandle, logins: Vec<String>, limit: usize) -> Vec<(String, Lines)> {
    let mut out = Vec::new();
    let mut missing = Vec::new();
    {
        let fetched = app.state::<Fetched>();
        let mut map = fetched.0.lock().unwrap_or_else(|p| p.into_inner());
        map.retain(|_, (at, _)| at.elapsed() < KEEP);
        for login in logins {
            match map.get(&login) {
                Some((_, lines)) => out.push((login, lines.clone())),
                None => missing.push(login),
            }
        }
    }
    let mut set = tokio::task::JoinSet::new();
    for login in missing {
        let app = app.clone();
        set.spawn(async move {
            let lines = tokio::time::timeout(WAIT, fetch(&app, &login, limit)).await.ok().flatten()?;
            Some((login, lines))
        });
    }
    let mut fresh = Vec::new();
    while let Some(done) = set.join_next().await {
        if let Ok(Some(got)) = done {
            fresh.push(got);
        }
    }
    if !fresh.is_empty() {
        let fetched = app.state::<Fetched>();
        let mut map = fetched.0.lock().unwrap_or_else(|p| p.into_inner());
        for (login, lines) in &fresh {
            map.insert(login.clone(), (Instant::now(), lines.clone()));
        }
        drop(map);
        // Let go of them once they're stale, searched again or not.
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(KEEP).await;
            let fetched = app.state::<Fetched>();
            fetched.0.lock().unwrap_or_else(|p| p.into_inner()).retain(|_, (at, _)| at.elapsed() < KEEP);
        });
    }
    out.extend(fresh);
    out
}

/// One channel's history lines, read the way the chat would show them (the user's rules
/// apply), each marked as history.
async fn fetch(app: &AppHandle, login: &str, limit: usize) -> Option<Lines> {
    let http = app.state::<crate::auth::Auth>().http.clone();
    let lines = cyex_twitch::recent::recent_messages(&http, login, limit as u32, Lane::Front).await.ok()?;
    let emotes = crate::emotes::room(app, None, login);
    let mut room = Room::new(login, None);
    room.set_keep(limit);
    room.set_rules(super::rules::current(app));
    Some(Arc::new(room.history(&lines, now_ms(), &emotes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_chat::irc::Line;
    use cyex_chat::tokenize::NoEmotes;

    fn room(lines: usize) -> Room {
        let mut r = Room::new("chan", None);
        r.set_keep(1000);
        for i in 0..lines {
            let raw = format!("@id=m{i};tmi-sent-ts={i};user-id=7 :a!a@a PRIVMSG #chan :hi");
            r.handle_line(&Line::parse(&raw).unwrap(), i as i64, &NoEmotes);
        }
        r
    }

    #[test]
    fn v107_only_background_rooms_that_dropped_lines_read_the_service() {
        let mut small = room(100);
        small.set_background(true);
        assert!(!cut(&small), "has all it ever had");
        let mut big = room(400);
        big.set_background(true);
        assert!(cut(&big));
        big.set_background(false);
        assert!(!cut(&big), "shown: it keeps the scrollback");
    }

    fn line(id: &str, ts: i64) -> FeedLine {
        let mut m = (*room(1).snapshot().messages[0]).clone();
        m.id = id.into();
        m.ts = ts;
        FeedLine { channel: "chan".into(), message: Arc::new(m), global: false }
    }

    #[test]
    fn v107_service_lines_merge_by_time_without_doubles() {
        // Kept lines first, then the service's: the overlap shows once.
        let out = finish(vec![line("b", 20), line("c", 30), line("a", 10), line("b", 20)]);
        let ids: Vec<&str> = out.iter().map(|l| l.message.id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
        let many: Vec<FeedLine> = (0..400).map(|i| line(&format!("m{i}"), i)).collect();
        let out = finish(many);
        assert_eq!((out.len(), out[0].message.id.as_str()), (MAX, "m100"), "the newest");
    }
}
