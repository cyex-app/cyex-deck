//! V108-BD Twitch badges uploaded while the app is open. A line that carries a badge its
//! catalog lacks makes the UI ask once (`chat_badge_missing`); that catalog is read again in
//! the background, at most once per 5 minutes each and one read at a time, and every window
//! gets the new one (`chat:catalog`), so lines already drawn show it. The global catalog is
//! also read again every 6 hours while it's loaded.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cyex_twitch::gql::SourceRoom;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::catalog::{self, ChannelCatalog, Catalogs, GlobalCatalog};

const EVENT: &str = "chat:catalog";
/// Least time between two reads of one catalog.
const GAP: Duration = Duration::from_secs(300);
/// The global catalog's slow re-read.
const GLOBAL_EVERY: Duration = Duration::from_secs(6 * 3600);
/// Catalogs tracked at most; past it only the ones waiting on a read stay.
const MAX_SLOTS: usize = 256;

/// Which catalog.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    Channel(String),
    Room(String),
}

/// What a re-read asked for does.
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    Now,
    /// The rest of the gap since the last read.
    After(Duration),
    /// One is already on its way.
    Skip,
}

#[derive(Default)]
struct Slot {
    last: Option<Instant>,
    busy: bool,
}

/// Last read and pending re-read per catalog.
#[derive(Default)]
pub struct Throttle {
    slots: HashMap<Scope, Slot>,
}

impl Throttle {
    /// A catalog was read (first load or re-read).
    pub fn read(&mut self, scope: Scope, now: Instant) {
        self.slot(scope).last = Some(now);
    }

    /// A re-read is wanted. Marks it on its way unless one already is.
    pub fn ask(&mut self, scope: Scope, now: Instant) -> Plan {
        let s = self.slot(scope);
        if s.busy {
            return Plan::Skip;
        }
        s.busy = true;
        match s.last.map(|t| now.saturating_duration_since(t)) {
            Some(gap) if gap < GAP => Plan::After(GAP - gap),
            _ => Plan::Now,
        }
    }

    /// The re-read finished or failed: the next one waits a full gap.
    pub fn done(&mut self, scope: Scope, now: Instant) {
        let s = self.slot(scope);
        s.busy = false;
        s.last = Some(now);
    }

    /// The channel closed: its slot goes, unless a re-read is on its way.
    pub fn forget(&mut self, scope: &Scope) {
        if self.slots.get(scope).is_some_and(|s| !s.busy) {
            self.slots.remove(scope);
        }
    }

    fn slot(&mut self, scope: Scope) -> &mut Slot {
        if self.slots.len() >= MAX_SLOTS && !self.slots.contains_key(&scope) {
            self.slots.retain(|_, s| s.busy);
        }
        self.slots.entry(scope).or_default()
    }
}

/// A catalog read again, for every window.
#[derive(Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum CatalogUpdate {
    Global { catalog: Arc<GlobalCatalog> },
    Channel { login: String, catalog: ChannelCatalog },
    /// A Shared Chat source room, by room id.
    Room { id: String, room: SourceRoom },
}

/// The slow global re-read.
pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(GLOBAL_EVERY).await;
            if app.state::<Catalogs>().global_cached().is_some() {
                reread(&app, Scope::Global);
            }
        }
    });
}

/// A line's badge isn't in `scope`'s catalog. Lines look in the global one second, so a
/// channel's or room's miss reads that one again too.
pub fn missing(app: &AppHandle, scope: Scope) {
    if scope != Scope::Global {
        reread(app, Scope::Global);
    }
    reread(app, scope);
}

fn reread(app: &AppHandle, scope: Scope) {
    let plan = app.state::<Catalogs>().throttle(|t| t.ask(scope.clone(), Instant::now()));
    let wait = match plan {
        Plan::Skip => return,
        Plan::Now => Duration::ZERO,
        Plan::After(d) => d,
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        let update = read(&app, &scope).await;
        app.state::<Catalogs>().throttle(|t| t.done(scope, Instant::now()));
        if let Some(u) = update {
            let _ = app.emit(EVENT, u);
        }
    });
}

/// Reads one catalog again; `None` when it failed or came back the same.
async fn read(app: &AppHandle, scope: &Scope) -> Option<CatalogUpdate> {
    match scope {
        Scope::Global => {
            let before = app.state::<Catalogs>().global_cached();
            let g = catalog::read_global(app).await.ok()?;
            let same = before.is_some_and(|b| b.badges == g.badges && b.cheermotes == g.cheermotes);
            (!same).then_some(CatalogUpdate::Global { catalog: g })
        }
        Scope::Channel(login) => {
            let before = catalog::cached(app, login);
            let c = catalog::channel(app, login, true).await.ok()??;
            let same = before.is_some_and(|b| b.badges == c.badges && b.cheermotes == c.cheermotes);
            (!same).then(|| CatalogUpdate::Channel {
                login: login.clone(),
                catalog: catalog::channel_view(&c),
            })
        }
        Scope::Room(id) => {
            let before = catalog::source_cached(app, id);
            let r = catalog::read_source(app, id).await.ok()?;
            let same = before.is_some_and(|b| *b == *r);
            (!same).then(|| CatalogUpdate::Room {
                id: id.clone(),
                room: r.as_ref().clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(login: &str) -> Scope {
        Scope::Channel(login.into())
    }

    #[test]
    fn never_read_goes_now_then_waits_for_the_first() {
        let mut t = Throttle::default();
        let now = Instant::now();
        assert_eq!(t.ask(ch("a"), now), Plan::Now);
        assert_eq!(t.ask(ch("a"), now), Plan::Skip);
        // Another catalog has its own slot.
        assert_eq!(t.ask(Scope::Global, now), Plan::Now);
    }

    #[test]
    fn a_recent_read_waits_out_the_gap() {
        let mut t = Throttle::default();
        let now = Instant::now();
        t.read(ch("a"), now);
        assert_eq!(t.ask(ch("a"), now + Duration::from_secs(60)), Plan::After(GAP - Duration::from_secs(60)));
        assert_eq!(t.ask(ch("a"), now + Duration::from_secs(61)), Plan::Skip);
        t.done(ch("a"), now + GAP);
        // Done starts the gap again.
        assert_eq!(t.ask(ch("a"), now + GAP + Duration::from_secs(1)), Plan::After(GAP - Duration::from_secs(1)));
    }

    #[test]
    fn an_old_read_goes_now() {
        let mut t = Throttle::default();
        let now = Instant::now();
        t.read(ch("a"), now);
        assert_eq!(t.ask(ch("a"), now + GAP), Plan::Now);
    }

    #[test]
    fn forget_keeps_a_read_on_its_way() {
        let mut t = Throttle::default();
        let now = Instant::now();
        t.read(ch("a"), now);
        t.forget(&ch("a"));
        assert_eq!(t.ask(ch("a"), now), Plan::Now);
        t.forget(&ch("a"));
        assert_eq!(t.ask(ch("a"), now), Plan::Skip);
    }

    #[test]
    fn slots_are_capped_but_pending_reads_stay() {
        let mut t = Throttle::default();
        let now = Instant::now();
        assert_eq!(t.ask(ch("busy"), now), Plan::Now);
        for i in 0..MAX_SLOTS * 2 {
            t.read(ch(&i.to_string()), now);
        }
        assert!(t.slots.len() <= MAX_SLOTS);
        assert_eq!(t.ask(ch("busy"), now), Plan::Skip);
    }
}
