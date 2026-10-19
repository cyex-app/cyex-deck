//! E1 A channel whose 7TV (E2), BTTV or FFZ emotes failed for now (429, 5xx, offline)
//! asks again later, a few times with growing waits. The tab on screen asks at once.
//! E2 The global 7TV set too, once per run.

use std::sync::Arc;
use std::time::Duration;

use cyex_chat::model::Provider;
use cyex_emotes::{EmoteSet, Scope};
use cyex_emotes::seventv;
use cyex_twitch::pace::{Lane, Service};
use tauri::{AppHandle, Emitter, Manager};

use super::providers::{self, Fetched, Which};
use super::{Emotes, load};

/// Waits before each later try, then it gives up (about 10 min in all).
const WAITS: [u64; 6] = [10, 20, 40, 80, 160, 300];
/// How often a waiting channel checks whether its tab came on screen.
const TICK: Duration = Duration::from_secs(2);

pub(super) fn spawn(app: AppHandle, login: String, id: String, failed: Which) {
    tauri::async_runtime::spawn(run(app, login, id, failed));
}

async fn run(app: AppHandle, login: String, id: String, mut failed: Which) {
    for (n, wait) in WAITS.iter().enumerate() {
        let lane = wait_turn(&app, &login, Duration::from_secs(*wait)).await;
        if !joined(&app, &login, &id) {
            return;
        }
        let http = app.state::<crate::auth::Auth>().http.clone();
        let got = providers::channel(&http, &id, lane, failed).await;
        let now = got.failed;
        let tries = n + 1;
        for (was, still, name) in [
            (failed.seventv, now.seventv, "7TV"),
            (failed.bttv, now.bttv, "BTTV"),
            (failed.ffz, now.ffz, "FFZ"),
        ] {
            if was && !still {
                eprintln!("emotes: {name} emotes for {id} in after {tries} retries");
            }
        }
        if !merge(&app, &login, &id, failed, got) {
            return;
        }
        failed = now;
        if !failed.any() {
            return;
        }
    }
    for (f, name) in [(failed.seventv, "7TV"), (failed.bttv, "BTTV"), (failed.ffz, "FFZ")] {
        if f {
            eprintln!("emotes: gave up on {name} emotes for {id}");
        }
    }
}

/// Waits `wait`, or less once the channel is on screen. Gives the lane to ask in.
async fn wait_turn(app: &AppHandle, login: &str, wait: Duration) -> Lane {
    let mut waited = Duration::ZERO;
    loop {
        let lane = crate::chat::shown::lane(app, login);
        if due(waited, wait, lane) {
            return lane;
        }
        tokio::time::sleep(TICK).await;
        waited += TICK;
    }
}

/// The tab on screen waits a fifth (at least a tick), so a burst of 429s isn't asked
/// again at once and its tries aren't spent in seconds.
fn due(waited: Duration, wait: Duration, lane: Lane) -> bool {
    waited >= wait || (lane == Lane::Front && waited >= (wait / 5).max(TICK))
}

fn joined(app: &AppHandle, login: &str, id: &str) -> bool {
    app.state::<Emotes>()
        .lock()
        .channels
        .get(login)
        .is_some_and(|c| c.id == id)
}

/// Puts what came in into the channel; false once the channel is gone.
fn merge(app: &AppHandle, login: &str, id: &str, asked: Which, got: Fetched) -> bool {
    let came = came(asked, got.failed);
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        let Some(c) = inner.channels.get_mut(login).filter(|c| c.id == id) else {
            return false;
        };
        if !came.any() {
            return true;
        }
        // No 7TV account: nothing to put in (none was there either).
        if came.seventv
            && let Some(ch) = got.seventv
        {
            load::put_seventv(&state.seventv, c, ch);
        }
        if came.bttv {
            replace(&mut c.sets, Provider::Bttv, Scope::Channel, got.bttv);
        }
        if came.ffz {
            replace(&mut c.sets, Provider::Ffz, Scope::Channel, got.ffz);
        }
        if let Some(b) = got.badges {
            c.ffz_badges = b;
        }
        inner.rebuild(login);
    }
    let _ = app.emit("emotes:loaded", login);
    true
}

/// What was asked for and answered this time (an answer may be "none").
fn came(asked: Which, failed: Which) -> Which {
    Which {
        seventv: asked.seventv && !failed.seventv,
        bttv: asked.bttv && !failed.bttv,
        ffz: asked.ffz && !failed.ffz,
    }
}

/// E2 The global 7TV set failed for now: asked again on the same waits, once per run
/// for the whole app. Stops when a later load brought it in.
pub(super) fn seventv_globals(app: AppHandle) {
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        if inner.seventv_globals_retry {
            return;
        }
        inner.seventv_globals_retry = true;
    }
    tauri::async_runtime::spawn(async move {
        for (n, wait) in WAITS.iter().enumerate() {
            tokio::time::sleep(Duration::from_secs(*wait)).await;
            if has_seventv_global(&app.state::<Emotes>().lock().globals) {
                return;
            }
            let http = app.state::<crate::auth::Auth>().http.clone();
            match seventv::api::global(&http).await {
                Ok(set) => {
                    eprintln!("emotes: 7TV globals in after {} retries", n + 1);
                    put_global(&app, set);
                    return;
                }
                Err(e) if Service::SevenTv.transient(&e) => {}
                Err(e) => {
                    eprintln!("emotes: no 7TV globals: {}", e.describe("7TV"));
                    return;
                }
            }
        }
        eprintln!("emotes: gave up on 7TV globals");
    });
}

fn has_seventv_global(globals: &[Arc<EmoteSet>]) -> bool {
    globals.iter().any(|s| s.provider == Provider::Seventv && s.scope == Scope::Global)
}

/// Puts the global 7TV set in and tells every chat.
fn put_global(app: &AppHandle, set: EmoteSet) {
    let logins: Vec<String> = {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        replace(&mut inner.globals, Provider::Seventv, Scope::Global, Some(Arc::new(set)));
        inner.rebuild_all();
        inner.channels.keys().cloned().collect()
    };
    for login in logins {
        let _ = app.emit("emotes:loaded", login);
    }
}

/// Swaps the provider's set of `scope` (BTTV's socket may have made one meanwhile).
fn replace(sets: &mut Vec<Arc<EmoteSet>>, provider: Provider, scope: Scope, set: Option<Arc<EmoteSet>>) {
    sets.retain(|s| !(s.provider == provider && s.scope == scope));
    sets.extend(set);
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: fn(u64) -> Duration = Duration::from_secs;

    #[test]
    fn background_waits_its_turn() {
        assert!(!due(S(0), S(10), Lane::Back));
        assert!(!due(S(8), S(10), Lane::Back));
        assert!(due(S(10), S(10), Lane::Back));
    }

    #[test]
    fn the_tab_on_screen_goes_first() {
        assert!(!due(S(0), S(10), Lane::Front), "a short pause after the failure");
        assert!(due(TICK, S(10), Lane::Front));
        assert!(!due(S(30), S(300), Lane::Front));
        assert!(due(S(60), S(300), Lane::Front));
    }

    #[test]
    fn retries_are_bounded() {
        assert!(WAITS.windows(2).all(|w| w[0] < w[1]));
        assert!(WAITS.iter().sum::<u64>() <= 15 * 60);
    }

    #[test]
    fn replace_swaps_only_that_providers_channel_set() {
        let set = |p, scope, id: &str| Arc::new(EmoteSet::new(id, p, scope, Vec::new()));
        let mut sets = vec![
            set(Provider::Seventv, Scope::Channel, "7"),
            set(Provider::Bttv, Scope::Channel, "old"),
            set(Provider::Ffz, Scope::Global, "g"),
        ];
        replace(&mut sets, Provider::Bttv, Scope::Channel, Some(set(Provider::Bttv, Scope::Channel, "new")));
        replace(&mut sets, Provider::Ffz, Scope::Channel, Some(set(Provider::Ffz, Scope::Channel, "room")));
        let ids: Vec<&str> = sets.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["7", "g", "new", "room"]);
    }

    #[test]
    fn the_global_7tv_set_is_swapped_not_doubled() {
        let set = |p, scope, id: &str| Arc::new(EmoteSet::new(id, p, scope, Vec::new()));
        let mut globals = vec![set(Provider::Bttv, Scope::Global, "b"), set(Provider::Seventv, Scope::Channel, "c")];
        assert!(!has_seventv_global(&globals), "a channel set isn't the global one");
        replace(&mut globals, Provider::Seventv, Scope::Global, Some(set(Provider::Seventv, Scope::Global, "g1")));
        replace(&mut globals, Provider::Seventv, Scope::Global, Some(set(Provider::Seventv, Scope::Global, "g2")));
        assert!(has_seventv_global(&globals));
        let ids: Vec<&str> = globals.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["b", "c", "g2"]);
    }

    #[test]
    fn only_answered_providers_are_put_in() {
        let asked = Which {
            seventv: true,
            bttv: false,
            ffz: true,
        };
        let failed = Which {
            seventv: false,
            bttv: false,
            ffz: true,
        };
        let got = came(asked, failed);
        assert!(got.seventv && !got.bttv && !got.ffz);
        assert!(!came(asked, asked).any(), "all still failing");
        assert!(came(Which::ALL, Which::default()) == Which::ALL);
    }
}
