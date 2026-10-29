//! V108-C01 Closed chats stay parked while the app runs, so reopening one shows what it had
//! (Chatterino keeps a channel's lines until it quits). A parked room is PARTed like any
//! closed one; opened again, it joins anew and the gap since it was parked fills from the
//! history service (`Room::resume`, C24). Hidden and parked rooms share one line budget
//! (`cyex_chat::budget`): over it, the oldest parked rooms go, then hidden ones shown
//! longest ago shrink.

use std::collections::HashMap;
use std::time::Duration;

use cyex_chat::Room;
use cyex_chat::budget::{self, Cut, Held};
use tauri::{AppHandle, Manager};

use super::{Chat, Identity, Inner, RoomKey, now_ms};

/// How often the budget is checked while lines come in.
const CHECK_EVERY: Duration = Duration::from_secs(10);

/// A closed room, when it was closed (ms), and whether its first history came in.
pub struct Parked {
    pub room: Room,
    pub at: i64,
    /// Without it (closed before it landed, or the load failed), it loads when restored.
    pub history: bool,
}

/// Closed rooms by key.
#[derive(Default)]
pub struct Lot {
    rooms: HashMap<RoomKey, Parked>,
}

impl Lot {
    /// Keeps a closed room. One that never got a line isn't worth it.
    pub fn park(&mut self, key: RoomKey, room: Room, now_ms: i64, history: bool) {
        if room.kept() > 0 {
            self.rooms.insert(key, Parked { room, at: now_ms, history });
        }
    }

    /// A parked room, taken out.
    pub fn take(&mut self, key: &RoomKey) -> Option<Parked> {
        self.rooms.remove(key)
    }

    /// A parked room, left in (a gap fill that came back after it was closed).
    pub fn get_mut(&mut self, key: &RoomKey) -> Option<&mut Room> {
        self.rooms.get_mut(key).map(|p| &mut p.room)
    }

    /// An account signed out: its rooms go, so signing in again starts clean.
    pub fn forget(&mut self, identity: &Identity) {
        self.rooms.retain(|(i, _), _| i != identity);
    }

    #[cfg(any(test, feature = "mocks"))]
    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    #[cfg(any(test, feature = "mocks"))]
    pub fn lines(&self) -> usize {
        self.rooms.values().map(|p| p.room.kept()).sum()
    }
}

/// Checks the budget every `CHECK_EVERY`.
pub fn init(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(CHECK_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            enforce(&mut app.state::<Chat>().lock());
        }
    });
}

/// Brings hidden and parked rooms under the line budget. Shown rooms don't count.
pub fn enforce(inner: &mut Inner) {
    let mut keys: Vec<(RoomKey, bool)> = Vec::new();
    let mut held: Vec<Held> = Vec::new();
    for (key, e) in inner.rooms.iter().filter(|(_, e)| e.views.is_empty()) {
        keys.push((key.clone(), false));
        held.push(Held {
            lines: e.room.kept(),
            last_shown: e.last_shown,
            parked: false,
            small: e.room.background(),
        });
    }
    for (key, p) in &inner.parked.rooms {
        keys.push((key.clone(), true));
        held.push(Held {
            lines: p.room.kept(),
            last_shown: p.at,
            parked: true,
            small: false,
        });
    }
    for cut in budget::cuts(&held, budget::LINE_BUDGET, budget::PARKED_MAX) {
        match cut {
            Cut::Shrink(i) => {
                if let Some(e) = inner.rooms.get_mut(&keys[i].0) {
                    e.room.set_background(true);
                }
            }
            Cut::Drop(i) => {
                inner.parked.rooms.remove(&keys[i].0);
            }
        }
    }
}

/// Parks a room that was just left and checks the budget.
pub fn park(inner: &mut Inner, key: RoomKey, room: Room, history: bool) {
    inner.parked.park(key, room, now_ms(), history);
    enforce(inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_chat::irc::Line;
    use cyex_chat::tokenize::NoEmotes;

    fn room(lines: usize) -> Room {
        let mut r = Room::new("chan", None);
        for i in 0..lines {
            let raw = format!("@id=m{i};tmi-sent-ts={i};user-id=7 :a!a@a PRIVMSG #chan :hi");
            r.handle_line(&Line::parse(&raw).unwrap(), i as i64, &NoEmotes);
        }
        r
    }

    fn key(login: &str) -> RoomKey {
        (Identity::Anon, login.into())
    }

    #[test]
    fn v108_01_only_rooms_with_lines_park() {
        let mut lot = Lot::default();
        lot.park(key("empty"), room(0), 1, true);
        lot.park(key("busy"), room(3), 2, false);
        assert_eq!((lot.len(), lot.lines()), (1, 3));
        let p = lot.take(&key("busy")).unwrap();
        assert_eq!((p.room.kept(), p.at, p.history), (3, 2, false));
        assert!(lot.take(&key("busy")).is_none(), "taken once");
    }

    #[test]
    fn v108_01_too_many_parked_drops_the_oldest() {
        let mut inner = Inner::default();
        for i in 0..=budget::PARKED_MAX {
            inner.parked.park(key(&format!("c{i}")), room(1), i as i64, true);
        }
        enforce(&mut inner);
        assert_eq!(inner.parked.len(), budget::PARKED_MAX);
        assert!(
            inner.parked.take(&key("c0")).is_none(),
            "closed first, gone first"
        );
        assert!(inner.parked.take(&key("c1")).is_some());
    }

    #[test]
    fn v108_01_signing_out_drops_that_accounts_rooms() {
        let mut lot = Lot::default();
        let acc = |l: &str| (Identity::Account("9".into()), l.to_owned());
        lot.park(acc("a"), room(1), 1, true);
        lot.park(key("a"), room(1), 1, true);
        lot.forget(&Identity::Account("9".into()));
        assert!(lot.take(&acc("a")).is_none());
        assert!(lot.take(&key("a")).is_some(), "anonymous ones stay");
    }

}
