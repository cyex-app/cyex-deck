//! V108-C31 FFZ has no live feed a third-party client can use (its pubsub is off), so each open
//! channel's FFZ room is read again every 10 minutes and after the chat reconnects. What changed
//! becomes chat lines (no one named: FFZ does not say) and the channel's set follows.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use cyex_chat::model::Provider;
use cyex_emotes::ffz::{self, diff};
use cyex_emotes::{Emote, Scope};
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};

use super::providers::{Fetched, Which};
use super::{Emotes, retry};

const EVERY: Duration = Duration::from_secs(600);
/// Between two channels, so many open chats don't ask FFZ at once.
const GAP: Duration = Duration::from_secs(2);

pub fn init(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(EVERY).await;
            let open: Vec<String> = {
                let state = app.state::<Emotes>();
                let inner = state.lock();
                inner.channels.keys().cloned().collect()
            };
            for login in open {
                check(&app, &login).await;
                tokio::time::sleep(GAP).await;
            }
        }
    });
}

/// The chat came back from a drop: read the channel's FFZ room again.
pub fn after_reconnect(app: &AppHandle, login: &str) {
    let app = app.clone();
    let login = login.to_owned();
    tauri::async_runtime::spawn(async move { check(&app, &login).await });
}

/// The channel's id, join generation and FFZ emotes now (if it has a set).
type Now = (String, u64, Option<Vec<Arc<Emote>>>);

fn current(app: &AppHandle, login: &str) -> Option<Now> {
    let state = app.state::<Emotes>();
    let inner = state.lock();
    let c = inner.channels.get(login)?;
    // Not loaded yet, or its first load is still being retried: nothing to compare with.
    if !*c.ready.borrow() || c.retry.as_ref().is_some_and(|t| !t.inner().is_finished()) {
        return None;
    }
    let old = c
        .sets
        .iter()
        .find(|s| s.provider == Provider::Ffz && s.scope == Scope::Channel)
        .map(|s| s.emotes.clone());
    Some((c.id.clone(), c.generation, old))
}

/// V108-F31 A channel with no FFZ set before and none now has nothing to put in or say.
fn nothing_to_read(had_set: bool, has_set: bool) -> bool {
    !had_set && !has_set
}

/// V108-F33 Channels being read right now: a reconnect inside the 10-minute round must not
/// read (and post the changes) a second time.
static READING: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Reading(String);

impl Reading {
    fn take(login: &str) -> Option<Self> {
        let mut on = READING.lock().unwrap_or_else(PoisonError::into_inner);
        if on.iter().any(|l| l == login) {
            return None;
        }
        on.push(login.to_owned());
        Some(Self(login.to_owned()))
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        let mut on = READING.lock().unwrap_or_else(PoisonError::into_inner);
        on.retain(|l| *l != self.0);
    }
}

async fn check(app: &AppHandle, login: &str) {
    let Some(_reading) = Reading::take(login) else {
        return;
    };
    let Some((id, generation, old)) = current(app, login) else {
        return;
    };
    let http = app.state::<crate::auth::Auth>().http.clone();
    let lane = crate::chat::shown::lane(app, login);
    let lane = if lane == Lane::Front { Lane::Front } else { Lane::Back };
    // A failed read is skipped; the next round asks again.
    let Ok(room) = ffz::api::room(&http, &id, lane).await else {
        return;
    };
    let (set, badges) = match room {
        Some(r) => (r.set.map(Arc::new), Some(r.badges)),
        None => (None, None),
    };
    if nothing_to_read(old.is_some(), set.is_some()) {
        return;
    }
    let new = set.as_ref().map(|s| s.emotes.clone()).unwrap_or_default();
    // Without an earlier set the room is new to us: put it in, say nothing.
    let changes = old.as_ref().map(|o| diff::diff(o, &new)).unwrap_or_default();
    if old.is_some() && changes.is_empty() {
        return;
    }
    let got = Fetched {
        ffz: set,
        badges,
        ..Fetched::default()
    };
    let only_ffz = Which {
        ffz: true,
        ..Which::default()
    };
    if retry::merge(app, login, generation, only_ffz, got) {
        for line in diff::lines(&changes) {
            super::change_line(app, login, &line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v108_f31_no_ffz_set_before_or_now_is_skipped() {
        assert!(nothing_to_read(false, false));
        assert!(!nothing_to_read(false, true));
        assert!(!nothing_to_read(true, false));
        assert!(!nothing_to_read(true, true));
    }

    #[test]
    fn v108_f33_one_read_per_channel_at_a_time() {
        let first = Reading::take("f33_a").expect("free");
        assert!(Reading::take("f33_a").is_none());
        assert!(Reading::take("f33_b").is_some());
        drop(first);
        assert!(Reading::take("f33_a").is_some());
    }
}
