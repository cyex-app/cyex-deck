//! V104-02 Global mentions. logs.cyex.app's firehose (refs 5.16) is read only while the
//! switch is on and a global mention has something to find. Every line is matched here
//! (`cyex_chat::mentions`); the list never leaves the app. Lines are peeked at without
//! parsing, the compiled list is swapped in without locking per line, and only a match
//! is parsed in full. Matches go to the Mentions tab marked global, with the same alerts
//! as a chat's own mentions.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_chat::build::{self, Colors, Ctx, Incoming};
use cyex_chat::irc::Line;
use cyex_chat::mentions::{self, MentionSet};
use cyex_chat::model::{Flag, Highlight, HighlightKind};
use cyex_chat::rules::Outcome;
use cyex_settings::Partition;
use cyex_settings::schema::Highlights;
use cyex_twitch::firehose::{self, Status};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;
use ts_rs::TS;

use crate::settings::Settings;

const EVENT: &str = "mentions:global";
/// More matches than this in a second are dropped: a pattern that broad would bury the
/// Mentions tab.
const MAX_PER_SECOND: u32 = 20;
/// Sounds, flashes and pop-ups from global mentions at most this often.
const ALERT_GAP: Duration = Duration::from_secs(3);

/// The firehose connection, for the settings page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export, rename = "GlobalStatus")]
pub enum GlobalStatus {
    /// Switched off, or nothing to look for.
    #[default]
    Off,
    Connecting,
    Connected,
    Offline {
        #[serde(rename = "retrySecs")]
        retry_secs: u32,
    },
}

#[derive(Default)]
pub struct Global {
    inner: Mutex<Inner>,
    shared: Arc<Shared>,
}

/// What the reader reads: the newest list, picked up when the generation moves, so no
/// line takes a lock.
#[derive(Default)]
struct Shared {
    set: Mutex<Arc<MentionSet>>,
    generation: AtomicU64,
}

#[derive(Default)]
struct Inner {
    stop: Option<watch::Sender<bool>>,
    /// Which reader may report its status: an old one still closing doesn't.
    run: u64,
    status: GlobalStatus,
}

impl Global {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
}

pub fn init(app: &AppHandle) {
    app.manage(Global::default());
    sync(app);
}

pub fn status(app: &AppHandle) -> GlobalStatus {
    app.try_state::<Global>()
        .map(|g| g.lock().status)
        .unwrap_or_default()
}

/// After the mentions changed: a new list, and the connection opened or closed.
pub fn sync(app: &AppHandle) {
    let Some(global) = app.try_state::<Global>() else {
        return;
    };
    let h: Highlights = app.state::<Settings>().read(Partition::Highlights);
    // V107-Mq Your names too, from every signed-in account.
    let accounts = crate::auth::accounts(app);
    let names: Vec<&str> = accounts.list.iter().map(|a| a.login.as_str()).collect();
    let set = MentionSet::new(&h.global_rules(&names));
    // Patterns that don't compile find nothing, so they don't keep it connected either.
    let active = !set.is_empty();
    *global.shared.set.lock().unwrap_or_else(|p| p.into_inner()) = Arc::new(set);
    global.shared.generation.fetch_add(1, Ordering::Release);
    let mut inner = global.lock();
    match (active, inner.stop.is_some()) {
        (true, false) => {
            let (tx, rx) = watch::channel(false);
            inner.stop = Some(tx);
            inner.run += 1;
            let run = inner.run;
            drop(inner);
            start(app.clone(), global.shared.clone(), rx, run);
        }
        (false, true) => {
            if let Some(stop) = inner.stop.take() {
                let _ = stop.send(true);
            }
            inner.run += 1;
            let run = inner.run;
            drop(inner);
            set_status(app, run, GlobalStatus::Off);
        }
        _ => {}
    }
}

fn set_status(app: &AppHandle, run: u64, status: GlobalStatus) {
    let Some(global) = app.try_state::<Global>() else {
        return;
    };
    {
        let mut inner = global.lock();
        if inner.run != run || inner.status == status {
            return;
        }
        inner.status = status;
    }
    let _ = app.emit(EVENT, status);
}

fn start(app: AppHandle, shared: Arc<Shared>, stop: watch::Receiver<bool>, run: u64) {
    let mut reader = Reader::new(app.clone(), shared);
    tauri::async_runtime::spawn(async move {
        firehose::run(
            firehose::URL,
            stop,
            |line| reader.line(line),
            |s| {
                let status = match s {
                    Status::Connecting => GlobalStatus::Connecting,
                    Status::Connected => GlobalStatus::Connected,
                    Status::Offline(secs) => GlobalStatus::Offline {
                        retry_secs: u32::try_from(secs).unwrap_or(u32::MAX),
                    },
                };
                set_status(&app, run, status);
            },
        )
        .await;
    });
}

/// What the reader keeps between lines.
struct Reader {
    app: AppHandle,
    shared: Arc<Shared>,
    set: Arc<MentionSet>,
    generation: u64,
    scratch: String,
    colors: Colors,
    second: Instant,
    in_second: u32,
    last_alert: Option<Instant>,
}

impl Reader {
    fn new(app: AppHandle, shared: Arc<Shared>) -> Self {
        Self {
            app,
            shared,
            set: Arc::default(),
            generation: u64::MAX,
            scratch: String::new(),
            colors: Colors::default(),
            second: Instant::now(),
            in_second: 0,
            last_alert: None,
        }
    }

    /// Runs for every line of the firehose: no allocation unless something matches.
    fn line(&mut self, raw: &str) {
        let generation = self.shared.generation.load(Ordering::Acquire);
        if generation != self.generation {
            self.set = self.shared.set.lock().unwrap_or_else(|p| p.into_inner()).clone();
            self.generation = generation;
        }
        let Some(peek) = mentions::peek(raw) else { return };
        let mut out = Outcome::default();
        let mut color: Option<&str> = None;
        self.set.each_match(peek.text, &mut self.scratch, |r| {
            color.get_or_insert(r.alert.color.as_str());
            out.add(&r.alert);
        });
        let Some(color) = color.map(str::to_owned) else {
            return;
        };
        if !self.room_for_one_more() {
            return;
        }
        self.matched(raw, peek.channel, color, out);
    }

    fn room_for_one_more(&mut self) -> bool {
        if self.second.elapsed() >= Duration::from_secs(1) {
            self.second = Instant::now();
            self.in_second = 0;
        }
        self.in_second += 1;
        self.in_second <= MAX_PER_SECOND
    }

    fn matched(&mut self, raw: &str, channel: &str, color: String, mut out: Outcome) {
        let Some(line) = Line::parse(raw) else { return };
        let mut ctx = Ctx::new(None, &mut self.colors, super::now_ms());
        let Some(Incoming::Message(mut m)) = build::build(&line, &mut ctx) else {
            return;
        };
        // Your own messages aren't mentions of you.
        let accounts = crate::auth::accounts(&self.app);
        if m.sender_id().is_some_and(|id| accounts.get(id).is_some()) {
            return;
        }
        // V107-Mq Never highlight holds here too.
        let rules = super::rules::current(&self.app);
        if m.sender.as_ref().is_some_and(|s| rules.never(&s.login)) {
            return;
        }
        m.set(Flag::Mention);
        m.meta.highlight = Some(Highlight {
            kind: HighlightKind::Mention,
            color: (!color.is_empty()).then_some(color),
        });
        let m = Arc::new(m);
        // V104-02 Always the Mentions tab (sanitize locks `show` on), plus any feed of its own.
        out.feed();
        super::feeds::route_global(&self.app, channel, &m, &out.routes);
        // C27 Silent chats stay silent here too; pings come at most every few seconds.
        let quiet = rules.muted(channel);
        let soon = self.last_alert.is_some_and(|t| t.elapsed() < ALERT_GAP);
        if out.pings() && !quiet && !soon {
            self.last_alert = Some(Instant::now());
            out.routes.clear();
            super::mention::notify(&self.app, channel, &m, &out, true);
        }
    }
}
