//! V110-U68 Hidden and idle: with every app window hidden (closed to the tray) and nothing
//! to earn, the miner asks Twitch nothing but Hermes for 5 minutes at a time. The channel
//! poll slows to 5 minutes, the watch loop parks, drops wait 30 minutes and the campaign
//! list waits unless badges are picked. The first 50 offline channels follow their stream
//! topic, so one going live wakes everything at once; so does a window showing.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use cyex_miner::cadence::{self, Cadence};
use cyex_twitch::hermes::Kind;
use tauri::{AppHandle, Manager};

use super::{Miner, State, lock};

/// Offline channels whose stream topic is followed while idle (a Hermes connection's worth).
const IDLE_TOPICS: usize = 50;

/// What the last look at the windows said; starts shown.
static VISIBLE: AtomicBool = AtomicBool::new(true);

/// Some app window (main, pop-out, overlay) is shown and not minimized.
pub fn visible(app: &AppHandle) -> bool {
    app.webview_windows()
        .values()
        .any(|w| crate::sys::paint::shown(&w.as_ref().window()))
}

/// The miner has an account and something to do: a slot in use, a stream playing here for
/// it (W1), or a badge.
pub(super) fn busy(st: &State) -> bool {
    st.account.is_some() && (!st.slots.is_empty() || !st.watching.is_empty() || st.focus.is_some())
}

/// How often the loops run now.
pub(super) fn cadence(app: &AppHandle) -> Cadence {
    let busy = busy(&lock(app));
    cadence::cadence(visible(app), busy)
}

/// Hidden with nothing to earn.
pub(super) fn idle(app: &AppHandle) -> bool {
    cadence(app).watch_park
}

/// A window was shown, hidden, minimized or closed (`windows::on_window_event`,
/// `windows::show_main`). Shown again after being hidden, every loop runs at once.
pub fn on_visibility(app: &AppHandle) {
    if app.try_state::<Miner>().is_none() {
        return;
    }
    let now = visible(app);
    let was = VISIBLE.swap(now, Ordering::Relaxed);
    if now == was {
        return;
    }
    if now {
        wake_all(app);
    }
    topics(app);
}

/// Wakes the channel, watch, drops and campaign loops.
pub(super) fn wake_all(app: &AppHandle) {
    if let Some(m) = app.try_state::<Miner>() {
        m.wake.notify_one();
        m.idle_wake.notify_waiters();
    }
}

/// Follows the stream topic of the first offline channels while idle, and lets go of
/// them otherwise.
pub(super) fn topics(app: &AppHandle) {
    let idle = idle(app);
    let (add, remove) = {
        let mut st = lock(app);
        let want = if idle && st.account.is_some() {
            wanted(&st)
        } else {
            HashSet::new()
        };
        let add: Vec<String> = want.difference(&st.idle_topics).cloned().collect();
        let remove: Vec<String> = st.idle_topics.difference(&want).cloned().collect();
        st.idle_topics = want;
        (add, remove)
    };
    for id in add {
        crate::hermes::add(app, Kind::Playback, &id);
    }
    for id in remove {
        crate::hermes::remove(app, Kind::Playback, &id);
    }
}

fn wanted(st: &State) -> HashSet<String> {
    let channels = st.channels.iter().map(|c| (c.id.as_str(), c.stream.is_some()));
    pick(channels, &st.followed)
}

/// The first [`IDLE_TOPICS`] offline channels not followed already.
fn pick<'a>(channels: impl Iterator<Item = (&'a str, bool)>, followed: &HashSet<String>) -> HashSet<String> {
    channels
        .filter(|(id, live)| !live && !followed.contains(*id))
        .take(IDLE_TOPICS)
        .map(|(id, _)| id.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v110_u68_idle_topics_take_the_first_fifty_offline_ones() {
        let ids: Vec<String> = (0..80).map(|i| i.to_string()).collect();
        let followed: HashSet<String> = ["0".to_owned()].into();
        let list = ids.iter().map(|i| (i.as_str(), i == "1"));
        let got = pick(list, &followed);
        assert_eq!(got.len(), IDLE_TOPICS);
        assert!(!got.contains("0") && !got.contains("1"));
        assert!(got.contains("2") && got.contains("51") && !got.contains("52"));
    }

    #[test]
    fn v110_u68_busy_needs_an_account_and_work() {
        let mut st = State {
            slots: vec!["a".into()],
            ..State::default()
        };
        assert!(!busy(&st));
        st.account = Some("me".into());
        assert!(busy(&st));
        st.slots.clear();
        assert!(!busy(&st));
        st.focus = Some(("a".into(), "c".into()));
        assert!(busy(&st));
    }
}
