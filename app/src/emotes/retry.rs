//! E1 A channel whose 7TV (E2), BTTV or FFZ emotes failed for now (429, 5xx, offline)
//! asks again later, a few times with growing waits. The tab on screen asks sooner.
//! B4 The global sets (and BTTV's and FFZ's badges) too, each provider that failed.
//! S2 These own background retries: background calls in `pace` give up early.

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cyex_chat::model::Provider;
use cyex_emotes::{EmoteSet, Scope};
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Emitter, Manager};

use super::providers::{self, Fetched, Which};
use super::{Channel, Emotes, load};

/// Waits before each later try, then it gives up (about 10 min in all).
const WAITS: [u64; 6] = [10, 20, 40, 80, 160, 300];
/// The tab on screen waits at least this before asking again.
const TICK: Duration = Duration::from_secs(2);

/// Starts the retries for join `generation`; B6 its leave stops them.
pub(super) fn spawn(app: AppHandle, login: String, id: String, generation: u64, failed: Which) {
    let task = tauri::async_runtime::spawn(run(app.clone(), login.clone(), id, generation, failed));
    let state = app.state::<Emotes>();
    let mut inner = state.lock();
    match inner.live(&login, generation) {
        Some(c) => c.retry = Some(task),
        None => task.abort(),
    }
}

async fn run(app: AppHandle, login: String, id: String, generation: u64, mut failed: Which) {
    for (n, wait) in WAITS.iter().enumerate() {
        let lane = wait_turn(&app, &login, Duration::from_secs(*wait)).await;
        if !joined(&app, &login, generation) {
            return;
        }
        let http = app.state::<crate::auth::Auth>().http.clone();
        let got = providers::channel(&http, &id, lane, failed).await;
        let now = got.failed;
        log_in(failed, now, &format!("emotes for {id}"), n + 1);
        if !merge(&app, &login, generation, failed, got) {
            return;
        }
        failed = now;
        if !failed.any() {
            return;
        }
    }
    log_gave_up(failed, &format!("emotes for {id}"));
}

fn log_in(was: Which, still: Which, what: &str, tries: usize) {
    for (name, came) in names(was.minus(still)) {
        if came {
            eprintln!("emotes: {name} {what} in after {tries} retries");
        }
    }
}

fn log_gave_up(failed: Which, what: &str) {
    for (name, f) in names(failed) {
        if f {
            eprintln!("emotes: gave up on {name} {what}");
        }
    }
}

fn names(w: Which) -> [(&'static str, bool); 3] {
    [("7TV", w.seventv), ("BTTV", w.bttv), ("FFZ", w.ffz)]
}

/// Waits `wait`, or less once the channel is on screen. Gives the lane to ask in. B5
/// Looks at the lane again only when views or tabs change, not on a timer.
async fn wait_turn(app: &AppHandle, login: &str, wait: Duration) -> Lane {
    let start = Instant::now();
    let mut wake = crate::chat::shown::wake(app);
    loop {
        let lane = crate::chat::shown::lane(app, login);
        let waited = start.elapsed();
        if due(waited, wait, lane) {
            return lane;
        }
        let left = until_due(waited, wait, lane);
        match &mut wake {
            Some(rx) => tokio::select! {
                () = tokio::time::sleep(left) => {}
                r = rx.changed() => {
                    if r.is_err() {
                        wake = None;
                    }
                }
            },
            None => tokio::time::sleep(left).await,
        }
    }
}

/// The tab on screen waits a fifth (at least a tick), so a burst of 429s isn't asked
/// again at once and its tries aren't spent in seconds.
fn due(waited: Duration, wait: Duration, lane: Lane) -> bool {
    waited >= wait || (lane == Lane::Front && waited >= front_wait(wait))
}

fn front_wait(wait: Duration) -> Duration {
    (wait / 5).max(TICK)
}

/// How long until `due` turns true in this lane.
fn until_due(waited: Duration, wait: Duration, lane: Lane) -> Duration {
    let at = match lane {
        Lane::Front => front_wait(wait).min(wait),
        Lane::Back => wait,
    };
    at.saturating_sub(waited)
}

/// B6 Still the same join, not just the same channel id.
fn joined(app: &AppHandle, login: &str, generation: u64) -> bool {
    app.state::<Emotes>().lock().live(login, generation).is_some()
}

/// Puts what came in into join `generation` of the channel and marks its emotes ready;
/// false once that join is gone. B1 The first load goes through here too: each answered
/// provider's set replaces the one a socket change or an earlier load put in.
pub(super) fn merge(app: &AppHandle, login: &str, generation: u64, asked: Which, mut got: Fetched) -> bool {
    let came = asked.minus(got.failed);
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        let Some(c) = inner.live(login, generation) else {
            return false;
        };
        let _ = c.ready.send(true);
        if !came.any() {
            return true;
        }
        // No 7TV account: nothing to put in (none was there either).
        if came.seventv
            && let Some(ch) = got.seventv.take()
        {
            load::put_seventv(&state.seventv, c, ch);
        }
        put(c, came, got);
        inner.rebuild(login);
    }
    // Windows refresh what depends on it, e.g. FFZ's mod and VIP pictures.
    let _ = app.emit("emotes:loaded", login);
    true
}

/// BTTV's and FFZ's answers into the channel, in place of what it had.
fn put(c: &mut Channel, came: Which, got: Fetched) {
    if came.bttv {
        replace(&mut c.sets, Provider::Bttv, Scope::Channel, got.bttv);
    }
    if came.ffz {
        replace(&mut c.sets, Provider::Ffz, Scope::Channel, got.ffz);
    }
    if let Some(b) = got.badges {
        c.ffz_badges = b;
    }
}

/// B4 Fetches the global sets in `want`, then asks again on the same waits for each
/// provider that failed for now. Clears `globals_loading` when done or given up, so a
/// later chat asks for what's still missing.
pub(super) fn globals(app: AppHandle, want: Which) {
    tauri::async_runtime::spawn(async move {
        let left = keep_asking(want, &WAITS, |ask, n| {
            let app = app.clone();
            async move {
                let http = app.state::<crate::auth::Auth>().http.clone();
                let got = providers::globals(&app, &http, ask).await;
                if n > 0 {
                    log_in(ask, got.failed, "globals", n);
                }
                put_globals(&app, got.answered, got.sets, n > 0);
                got.failed
            }
        })
        .await;
        log_gave_up(left, "globals");
        app.state::<Emotes>().lock().globals_loading = false;
    });
}

/// Asks for `want`, then again after each of `waits` for what failed (`ask` gives what
/// failed, and gets the try's number). What still failed at the end.
async fn keep_asking<F, Fut>(mut want: Which, waits: &[u64], mut ask: F) -> Which
where
    F: FnMut(Which, usize) -> Fut,
    Fut: Future<Output = Which>,
{
    for n in 0..=waits.len() {
        if n > 0 {
            tokio::time::sleep(Duration::from_secs(waits[n - 1])).await;
        }
        want = want.minus(want.minus(ask(want, n).await));
        if !want.any() {
            break;
        }
    }
    want
}

/// The answered providers' global sets in place of theirs; tells every chat on a retry.
fn put_globals(app: &AppHandle, answered: Which, sets: Vec<Arc<EmoteSet>>, tell: bool) {
    let state = app.state::<Emotes>();
    let logins: Vec<String> = {
        let mut inner = state.lock();
        swap_globals(&mut inner.globals, answered, sets);
        inner.globals_done = inner.globals_done.or(answered);
        if answered.any() {
            inner.rebuild_all();
        }
        if tell && answered.any() {
            inner.channels.keys().cloned().collect()
        } else {
            Vec::new()
        }
    };
    let _ = state.globals_ready.send(true);
    for login in logins {
        let _ = app.emit("emotes:loaded", login);
    }
}

/// Each answered provider's global sets (FFZ has a few) in place of its old ones.
fn swap_globals(globals: &mut Vec<Arc<EmoteSet>>, answered: Which, sets: Vec<Arc<EmoteSet>>) {
    let swapped = |p: Provider| match p {
        Provider::Seventv => answered.seventv,
        Provider::Bttv => answered.bttv,
        Provider::Ffz => answered.ffz,
        Provider::Twitch => false,
    };
    globals.retain(|s| !swapped(s.provider));
    globals.extend(sets.into_iter().filter(|s| swapped(s.provider)));
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

    fn set(p: Provider, scope: Scope, id: &str) -> Arc<EmoteSet> {
        Arc::new(EmoteSet::new(id, p, scope, Vec::new()))
    }

    #[test]
    fn background_waits_its_turn() {
        assert!(!due(S(0), S(10), Lane::Back));
        assert!(!due(S(8), S(10), Lane::Back));
        assert!(due(S(10), S(10), Lane::Back));
        assert_eq!(until_due(S(4), S(10), Lane::Back), S(6));
    }

    #[test]
    fn the_tab_on_screen_goes_first() {
        assert!(!due(S(0), S(10), Lane::Front), "a short pause after the failure");
        assert!(due(TICK, S(10), Lane::Front));
        assert!(!due(S(30), S(300), Lane::Front));
        assert!(due(S(60), S(300), Lane::Front));
        assert_eq!(until_due(S(30), S(300), Lane::Front), S(30));
        assert_eq!(until_due(S(1), S(10), Lane::Front), S(1));
    }

    #[test]
    fn retries_are_bounded() {
        assert!(WAITS.windows(2).all(|w| w[0] < w[1]));
        assert!(WAITS.iter().sum::<u64>() <= 15 * 60);
    }

    #[test]
    fn replace_swaps_only_that_providers_channel_set() {
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

    /// B1 A BTTV socket change made `bttv-{id}` before the first load: the load replaces
    /// it, and a second load (a quick rejoin) doesn't double it.
    #[test]
    fn b1_a_socket_set_before_the_first_load_is_not_doubled() {
        let mut c = Channel {
            id: "1".into(),
            generation: 1,
            retry: None,
            sets: vec![set(Provider::Bttv, Scope::Channel, "bttv-1")],
            seventv_user: None,
            ffz_badges: Default::default(),
            lookup: Arc::default(),
            ready: tokio::sync::watch::channel(false).0,
        };
        let fetched = || Fetched {
            bttv: Some(set(Provider::Bttv, Scope::Channel, "real")),
            ffz: Some(set(Provider::Ffz, Scope::Channel, "room")),
            ..Fetched::default()
        };
        put(&mut c, Which::ALL, fetched());
        put(&mut c, Which::ALL, fetched());
        let ids: Vec<&str> = c.sets.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["real", "room"]);
        // A provider that failed keeps what it had.
        let failed = Fetched {
            failed: Which { bttv: true, ..Which::default() },
            ..Fetched::default()
        };
        put(&mut c, Which::ALL.minus(failed.failed), failed);
        let ids: Vec<&str> = c.sets.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["real"], "FFZ answered no room");
    }

    #[test]
    fn b4_global_sets_swap_per_provider() {
        let mut g = vec![set(Provider::Bttv, Scope::Global, "b"), set(Provider::Ffz, Scope::Global, "f1")];
        let ffz = Which { ffz: true, ..Which::default() };
        swap_globals(
            &mut g,
            ffz,
            vec![set(Provider::Ffz, Scope::Global, "f2"), set(Provider::Ffz, Scope::Global, "f3")],
        );
        swap_globals(&mut g, Which { seventv: true, ..Which::default() }, vec![set(Provider::Seventv, Scope::Global, "7")]);
        swap_globals(&mut g, Which { seventv: true, ..Which::default() }, vec![set(Provider::Seventv, Scope::Global, "7b")]);
        let ids: Vec<&str> = g.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["b", "f2", "f3", "7b"]);
    }

    /// B4 Every global provider that failed for now is asked again, only those.
    #[tokio::test]
    async fn b4_failed_globals_are_asked_again() {
        let mut asked = Vec::new();
        let ffz = Which { ffz: true, ..Which::default() };
        let bttv_ffz = Which { bttv: true, ffz: true, ..Which::default() };
        let mut fails = vec![bttv_ffz, ffz, Which::default()].into_iter();
        let left = keep_asking(Which::ALL, &[0, 0, 0, 0], |w, n| {
            asked.push((w, n));
            let f = fails.next().unwrap();
            async move { f }
        })
        .await;
        assert_eq!(asked, [(Which::ALL, 0), (bttv_ffz, 1), (ffz, 2)]);
        assert!(!left.any());
        // Gives up after the last wait, with what still failed.
        let mut tries = 0;
        let left = keep_asking(ffz, &[0, 0], |_, _| {
            tries += 1;
            async move { ffz }
        })
        .await;
        assert_eq!((tries, left), (3, ffz));
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
        let got = asked.minus(failed);
        assert!(got.seventv && !got.bttv && !got.ffz);
        assert!(!asked.minus(asked).any(), "all still failing");
        assert!(Which::ALL.minus(Which::default()) == Which::ALL);
    }
}
