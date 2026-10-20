//! V107-R16 The joined channels, one place: login → channel id, join generation and how
//! many rooms (one per identity) and views use it. Joins and leaves go through here; other
//! modules hook in instead of pruning their own per-channel maps.
//!
//! Interface:
//! - [`Registry::room_opened`] gives the join's generation. An async load captures it and
//!   reports the channel id with [`Registry::known`], which says no once the channel was left
//!   (or left and joined again), so a late result is dropped instead of undone.
//! - Hooks get every [`Event`] once: `Known` (first id of a generation), `Gone` (last room
//!   left), `Shown` (first view), `Hidden` (last view). Events are fired one at a time, in
//!   order, after the state changed: a hook that checks [`Registry::joined`] sees the new
//!   state. Hooks must not call the mutating methods here, nor take the chat lock (leaves fire
//!   under it).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, RwLock};

pub enum Event<'a> {
    Known { login: &'a str, id: &'a str, generation: u64 },
    Gone { login: &'a str, id: Option<&'a str>, generation: u64 },
    Shown { login: &'a str, account: Option<&'a str> },
    Hidden { login: &'a str },
}

pub type Hook<C> = fn(&C, &Event);

pub struct Registry<C> {
    state: Mutex<State>,
    /// Held while a change fires its hooks, so two changes' events never interleave.
    dispatch: Mutex<()>,
    hooks: RwLock<Vec<Hook<C>>>,
}

#[derive(Default)]
struct State {
    next_gen: u64,
    by_login: HashMap<String, Joined>,
}

#[derive(Default)]
struct Joined {
    generation: u64,
    id: Option<String>,
    rooms: usize,
    views: usize,
}

impl<C> Default for Registry<C> {
    fn default() -> Self {
        Self {
            state: Mutex::default(),
            dispatch: Mutex::new(()),
            hooks: RwLock::default(),
        }
    }
}

impl<C> Registry<C> {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn dispatch(&self) -> MutexGuard<'_, ()> {
        self.dispatch.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn fire(&self, ctx: &C, e: &Event) {
        for h in self.hooks.read().unwrap_or_else(|p| p.into_inner()).iter() {
            h(ctx, e);
        }
    }

    pub fn hook(&self, f: Hook<C>) {
        self.hooks.write().unwrap_or_else(|p| p.into_inner()).push(f);
    }

    /// A room of `login` opened. The channel's join generation (a new one for its first room).
    pub fn room_opened(&self, login: &str) -> u64 {
        let _d = self.dispatch();
        let mut s = self.state();
        let fresh = s.by_login.get(login).is_none_or(|j| j.rooms == 0);
        if fresh {
            s.next_gen += 1;
        }
        let next = s.next_gen;
        let j = s.by_login.entry(login.to_owned()).or_default();
        if fresh {
            j.generation = next;
            j.id = None;
        }
        j.rooms += 1;
        j.generation
    }

    /// A room of `login` closed; its last one fires `Gone`.
    pub fn room_closed(&self, ctx: &C, login: &str) {
        let _d = self.dispatch();
        let gone = {
            let mut s = self.state();
            let Some(j) = s.by_login.get_mut(login) else { return };
            if j.rooms == 0 {
                return;
            }
            j.rooms -= 1;
            if j.rooms > 0 {
                return;
            }
            let gone = (j.generation, j.id.take());
            if j.views == 0 {
                s.by_login.remove(login);
            }
            gone
        };
        self.fire(ctx, &Event::Gone { login, id: gone.1.as_deref(), generation: gone.0 });
    }

    /// A load for join `generation` found the channel id. `false`: that join is over, drop
    /// the result. The first id of a join fires `Known`.
    pub fn known(&self, ctx: &C, login: &str, generation: u64, id: &str) -> bool {
        let _d = self.dispatch();
        {
            let mut s = self.state();
            let Some(j) = s.by_login.get_mut(login).filter(|j| j.rooms > 0 && j.generation == generation) else {
                return false;
            };
            if j.id.is_some() {
                return true;
            }
            j.id = Some(id.to_owned());
        }
        self.fire(ctx, &Event::Known { login, id, generation });
        true
    }

    /// Something shows `login` (a chat view, the stream manager); the first fires `Shown`.
    pub fn view_opened(&self, ctx: &C, login: &str, account: Option<&str>) {
        let _d = self.dispatch();
        let first = {
            let mut s = self.state();
            let j = s.by_login.entry(login.to_owned()).or_default();
            j.views += 1;
            j.views == 1
        };
        if first {
            self.fire(ctx, &Event::Shown { login, account });
        }
    }

    /// The last view of `login` closing fires `Hidden`.
    pub fn view_closed(&self, ctx: &C, login: &str) {
        let _d = self.dispatch();
        {
            let mut s = self.state();
            let Some(j) = s.by_login.get_mut(login) else { return };
            if j.views == 0 {
                return;
            }
            j.views -= 1;
            if j.views > 0 {
                return;
            }
            if j.rooms == 0 {
                s.by_login.remove(login);
            }
        }
        self.fire(ctx, &Event::Hidden { login });
    }

    /// Join `generation` of `login` is still on.
    pub fn current(&self, login: &str, generation: u64) -> bool {
        self.state()
            .by_login
            .get(login)
            .is_some_and(|j| j.rooms > 0 && j.generation == generation)
    }

    /// Some room has `login` joined.
    pub fn joined(&self, login: &str) -> bool {
        self.state().by_login.get(login).is_some_and(|j| j.rooms > 0)
    }

    /// The channel id of a joined channel, once a load found it.
    pub fn id(&self, login: &str) -> Option<String> {
        self.state().by_login.get(login).and_then(|j| j.id.clone())
    }

    /// The joined channel with this id.
    pub fn login_of(&self, id: &str) -> Option<String> {
        self.state()
            .by_login
            .iter()
            .find(|(_, j)| j.id.as_deref() == Some(id))
            .map(|(l, _)| l.clone())
    }

    /// Channels with a room or a view, for the memory probe.
    #[cfg(any(test, feature = "mocks"))]
    pub fn count(&self) -> usize {
        self.state().by_login.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Log = Mutex<Vec<String>>;

    fn log(ctx: &Log, e: &Event) {
        let line = match e {
            Event::Known { login, id, generation } => format!("known {login} {id} {generation}"),
            Event::Gone { login, id, generation } => format!("gone {login} {id:?} {generation}"),
            Event::Shown { login, account } => format!("shown {login} {account:?}"),
            Event::Hidden { login } => format!("hidden {login}"),
        };
        ctx.lock().unwrap().push(line);
    }

    fn reg() -> (Registry<Log>, Log) {
        let r = Registry::default();
        r.hook(log);
        (r, Log::default())
    }

    #[test]
    fn v107_r16_a_late_load_after_leave_is_dropped() {
        let (r, ctx) = reg();
        let g = r.room_opened("a");
        r.room_closed(&ctx, "a");
        assert!(!r.known(&ctx, "a", g, "1"));
        assert_eq!(r.id("a"), None);
        assert_eq!(*ctx.lock().unwrap(), ["gone a None 1"]);
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn v107_r16_rejoin_gets_a_new_generation() {
        let (r, ctx) = reg();
        let g1 = r.room_opened("a");
        assert!(r.known(&ctx, "a", g1, "1"));
        r.room_closed(&ctx, "a");
        let g2 = r.room_opened("a");
        assert_ne!(g1, g2);
        assert!(!r.current("a", g1) && r.current("a", g2));
        assert!(!r.known(&ctx, "a", g1, "1"), "the first join's load is stale");
        assert_eq!(r.id("a"), None);
        assert!(r.known(&ctx, "a", g2, "1"));
        assert_eq!(r.login_of("1").as_deref(), Some("a"));
    }

    #[test]
    fn v107_r16_every_hook_fires_once() {
        let (r, ctx) = reg();
        let g = r.room_opened("a");
        assert_eq!(r.room_opened("a"), g, "a second identity shares the join");
        assert!(r.known(&ctx, "a", g, "1") && r.known(&ctx, "a", g, "1"));
        r.view_opened(&ctx, "a", Some("me"));
        r.view_opened(&ctx, "a", None);
        r.view_closed(&ctx, "a");
        r.room_closed(&ctx, "a");
        r.view_closed(&ctx, "a");
        r.view_closed(&ctx, "a");
        r.room_closed(&ctx, "a");
        r.room_closed(&ctx, "a");
        assert_eq!(
            *ctx.lock().unwrap(),
            ["known a 1 1", "shown a Some(\"me\")", "hidden a", "gone a Some(\"1\") 1"]
        );
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn v107_r16_a_view_without_a_room_is_not_joined() {
        let (r, ctx) = reg();
        r.view_opened(&ctx, "a", None);
        assert!(!r.joined("a"));
        let g = r.room_opened("a");
        assert!(r.known(&ctx, "a", g, "1"));
        r.room_closed(&ctx, "a");
        assert!(!r.joined("a") && r.id("a").is_none() && r.count() == 1);
        r.view_closed(&ctx, "a");
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn v107_r16_a_thousand_joins_and_leaves_leave_nothing() {
        let (r, ctx) = reg();
        r.room_opened("stay");
        for i in 0..1000 {
            let login = format!("c{i}");
            let g = r.room_opened(&login);
            r.view_opened(&ctx, &login, None);
            r.known(&ctx, &login, g, &i.to_string());
            r.view_closed(&ctx, &login);
            r.room_closed(&ctx, &login);
        }
        assert_eq!(r.count(), 1);
        assert_eq!(ctx.lock().unwrap().len(), 4000);
    }
}
