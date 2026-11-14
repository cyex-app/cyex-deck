//! V104-02 Global mentions. logs.cyex.app's firehose (refs 5.16) is read only while the
//! switch is on and a global mention has something to find. Every line is matched here
//! (`cyex_chat::mentions`); the list never leaves the app. Lines are peeked at without
//! parsing, the compiled list is swapped in without locking per line, and only a match
//! is parsed in full. Matches go to the Mentions tab and/or a custom feed (V107-H16, per
//! row) marked global, with the same alerts as a chat's own mentions.
//! V111-12 The same list also runs on every live line of the open chats and of the
//! background reads of channels where the account has a role (`role_reads`), so those come
//! straight from Twitch, not through logs.cyex.app (which lags a little, and gets Twitch's
//! chat delay where you'd skip it). Whichever copy of a message comes first counts; later
//! ones are dropped by id, and one alert gap covers every path.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_chat::build::{self, Colors, Ctx, Incoming};
use cyex_chat::irc::Line;
use cyex_chat::mentions::{self, MentionSet};
use cyex_chat::model::{ChatMessage, Flag, Highlight, HighlightKind};
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
/// V108-GM The feed behind the Global mentions tab in the title bar.
pub const TAB_FEED: &str = "Global mentions";
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
    /// V108-GM Where a row's "tab" choice goes: the Global mentions tab or the chosen feed.
    dest: Mutex<Option<Arc<str>>>,
    generation: AtomicU64,
    /// V111-12 The list has something to find: chat lines skip everything else when not.
    active: AtomicBool,
    /// V111-12 Messages already taken and the last alert, for every path at once.
    gate: Mutex<Gate>,
    /// V111-12 The reader for chat lines (open chats and role reads), made on first use.
    irc: Mutex<Option<Reader>>,
}

/// V111-12 One message counts once, whichever path brings it first (the firehose, an open
/// chat, a role read); alerts at most every [`ALERT_GAP`] across all of them.
#[derive(Default)]
struct Gate {
    seen: VecDeque<(String, Instant)>,
    last_alert: Option<Instant>,
}

impl Gate {
    /// False when this message id was taken already.
    fn first(&mut self, id: &str, now: Instant) -> bool {
        super::mention::remember(&mut self.seen, id, now)
    }

    /// Whether an alert may go now; if so, it counts as the last one.
    fn alert(&mut self, now: Instant) -> bool {
        if self.last_alert.is_some_and(|t| now.saturating_duration_since(t) < ALERT_GAP) {
            return false;
        }
        self.last_alert = Some(now);
        true
    }
}

#[derive(Default)]
struct Inner {
    stop: Option<watch::Sender<bool>>,
    /// Which reader may report its status: an old one still closing doesn't.
    run: u64,
    status: GlobalStatus,
    /// V108-GM The switch at the last sync: turning it on adds your name as a row.
    was_on: Option<bool>,
}

impl Global {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
}

pub fn init(app: &AppHandle) {
    app.manage(Global::default());
    super::role_reads::init(app);
    sync(app);
}

/// V111-12 A raw line off a chat connection (an open chat's or a role read's): matched
/// like a firehose line. Anything but a chat message costs a peek.
pub fn irc_line(app: &AppHandle, raw: &str) {
    let Some(global) = app.try_state::<Global>() else { return };
    let shared = &global.shared;
    if !shared.active.load(Ordering::Acquire) {
        return;
    }
    let mut irc = shared.irc.lock().unwrap_or_else(|p| p.into_inner());
    irc.get_or_insert_with(|| Reader::new(app.clone())).line(shared, raw);
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
    if name_row(app, &global, &h) {
        return; // saved: the change syncs again
    }
    let set = MentionSet::new(&h.global_rules());
    *global.shared.dest.lock().unwrap_or_else(|p| p.into_inner()) = destination(&h);
    // Patterns that don't compile find nothing, so they don't keep it connected either.
    let active = !set.is_empty();
    *global.shared.set.lock().unwrap_or_else(|p| p.into_inner()) = Arc::new(set);
    global.shared.generation.fetch_add(1, Ordering::Release);
    global.shared.active.store(active, Ordering::Release);
    // V111-12 Role channels are read while there is something to find.
    super::role_reads::sync(app, active);
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

/// V108-GM Switched on (or an older file, once): the default account's name becomes a row
/// of the list, which people can see and remove. True when that was saved.
fn name_row(app: &AppHandle, global: &Global, h: &Highlights) -> bool {
    let turned_on = {
        let mut inner = global.lock();
        let was = inner.was_on.replace(h.global);
        h.global && was == Some(false)
    };
    if !h.global || (h.global_named && !turned_on) {
        return false;
    }
    let accounts = crate::auth::accounts(app);
    // Nobody signed in yet: the next account change tries again.
    let Some(login) = accounts.default_account().map(|a| a.login.clone()) else { return false };
    // Only a real save syncs again: switched back on with the row still there, nothing is.
    let mut saved = false;
    let ok = app
        .state::<Settings>()
        .update::<Highlights>(app, Partition::Highlights, |h| {
            let added = h.add_global_name(&login);
            let named = !std::mem::replace(&mut h.global_named, true);
            saved = added || named;
            saved
        })
        .is_ok();
    ok && saved
}

/// V108-GM The tab when it's on, else the feed picked for them (none: rows' own feeds only).
fn destination(h: &Highlights) -> Option<Arc<str>> {
    if h.global_tab {
        Some(TAB_FEED.into())
    } else {
        Some(h.global_feed.trim()).filter(|f| !f.is_empty()).map(Arc::from)
    }
}

/// A row's "Mentions" choice becomes `dest`; feeds of its own stay.
fn place(routes: Vec<String>, dest: Option<&str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in routes {
        let r = if r.trim().eq_ignore_ascii_case(mentions::FEED) {
            match dest {
                Some(d) => d.to_owned(),
                None => continue,
            }
        } else {
            r
        };
        if !out.iter().any(|x| x.trim().eq_ignore_ascii_case(r.trim())) {
            out.push(r);
        }
    }
    out
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
    let mut reader = Reader::new(app.clone());
    tauri::async_runtime::spawn(async move {
        firehose::run(
            firehose::URL,
            stop,
            |line| reader.line(&shared, line),
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

/// V107-H16 What a line's matches ask for: the first match's color, every match's alerts
/// and tabs (`show` the Mentions tab, `route` a custom feed). None: no match, or one that
/// goes nowhere and pings nothing, so the line isn't parsed.
fn outcome(set: &MentionSet, text: &str, scratch: &mut String) -> Option<(String, Outcome)> {
    let mut out = Outcome::default();
    let mut color: Option<&str> = None;
    set.each_match(text, scratch, |r| {
        color.get_or_insert(r.alert.color.as_str());
        out.add(&r.alert);
    });
    let color = color?.to_owned();
    (!out.routes.is_empty() || out.pings()).then_some((color, out))
}

/// A matched line as a message, marked as a mention in `color`. None: not a chat message,
/// your own (`own` sender id), or from someone never highlighted (`never` login).
fn mention_of(
    raw: &str,
    color: String,
    colors: &mut Colors,
    own: impl Fn(&str) -> bool,
    never: impl Fn(&str) -> bool,
) -> Option<ChatMessage> {
    let line = Line::parse(raw)?;
    let mut ctx = Ctx::new(None, colors, super::now_ms());
    let Some(Incoming::Message(mut m)) = build::build(&line, &mut ctx) else {
        return None;
    };
    // Your own messages aren't mentions of you.
    if m.sender_id().is_some_and(&own) {
        return None;
    }
    // V107-Mq Never highlight holds here too.
    if m.sender.as_ref().is_some_and(|s| never(&s.login)) {
        return None;
    }
    m.set(Flag::Mention);
    m.meta.highlight = Some(Highlight {
        kind: HighlightKind::Mention,
        color: (!color.is_empty()).then_some(color),
        words: None,
    });
    Some(m)
}

/// What a reader keeps between lines: one for the firehose, one for chat lines.
struct Reader {
    app: AppHandle,
    set: Arc<MentionSet>,
    dest: Option<Arc<str>>,
    generation: u64,
    scratch: String,
    colors: Colors,
    second: Instant,
    in_second: u32,
}

impl Reader {
    fn new(app: AppHandle) -> Self {
        Self {
            app,
            set: Arc::default(),
            dest: None,
            generation: u64::MAX,
            scratch: String::new(),
            colors: Colors::default(),
            second: Instant::now(),
            in_second: 0,
        }
    }

    /// Runs for every line: no allocation unless something matches.
    fn line(&mut self, shared: &Shared, raw: &str) {
        let generation = shared.generation.load(Ordering::Acquire);
        if generation != self.generation {
            self.set = shared.set.lock().unwrap_or_else(|p| p.into_inner()).clone();
            self.dest = shared.dest.lock().unwrap_or_else(|p| p.into_inner()).clone();
            self.generation = generation;
        }
        let Some(peek) = mentions::peek(raw) else { return };
        let Some((color, out)) = outcome(&self.set, peek.text, &mut self.scratch) else {
            return;
        };
        if !self.room_for_one_more() {
            return;
        }
        self.matched(shared, raw, peek.channel, color, out);
    }

    fn room_for_one_more(&mut self) -> bool {
        if self.second.elapsed() >= Duration::from_secs(1) {
            self.second = Instant::now();
            self.in_second = 0;
        }
        self.in_second += 1;
        self.in_second <= MAX_PER_SECOND
    }

    fn matched(&mut self, shared: &Shared, raw: &str, channel: &str, color: String, mut out: Outcome) {
        let accounts = crate::auth::accounts(&self.app);
        let rules = super::rules::current(&self.app);
        let own = |id: &str| accounts.get(id).is_some();
        let Some(m) = mention_of(raw, color, &mut self.colors, own, |login| rules.never(login)) else {
            return;
        };
        // V111-12 The first copy of a message counts: the firehose's, an open chat's or a role read's.
        if !shared.gate.lock().unwrap_or_else(|p| p.into_inner()).first(&m.id, Instant::now()) {
            return;
        }
        let m = Arc::new(m);
        // V107-H16 V108-GM The global tab (or its feed) when the row asks for it, plus any feed of its own.
        out.routes = place(std::mem::take(&mut out.routes), self.dest.as_deref());
        super::feeds::route_global(&self.app, channel, &m, &out.routes);
        // C27 Silent chats stay silent here too; pings come at most every few seconds.
        if out.pings()
            && !rules.muted(channel)
            && shared.gate.lock().unwrap_or_else(|p| p.into_inner()).alert(Instant::now())
        {
            out.routes.clear();
            // V111-12 A chat that is open pops up as that chat; any other as its #channel window.
            let global = !super::is_open(&self.app, channel);
            super::mention::notify(&self.app, channel, &m, &out, global);
        }
    }
}

#[cfg(test)]
mod tests {
    use cyex_settings::schema::{Alert, PhraseRule};

    use super::*;

    fn row(pattern: &str, show: bool, route: &str) -> PhraseRule {
        PhraseRule {
            pattern: pattern.into(),
            alert: Alert {
                show,
                route: route.into(),
                ..Alert::default()
            },
            ..PhraseRule::default()
        }
    }

    fn routes(rows: &[PhraseRule], text: &str) -> Option<Vec<String>> {
        let h = Highlights {
            global: true,
            global_list: rows.to_vec(),
            ..Highlights::default()
        };
        let set = MentionSet::new(&h.global_rules());
        outcome(&set, text, &mut String::new()).map(|(_, o)| o.routes)
    }

    // V107-H16 Mentions on, off, a custom feed, a feed plus a copy in Mentions.
    #[test]
    fn global_rows_route_where_they_ask() {
        assert_eq!(routes(&[row("deck", true, "")], "cyex deck").unwrap(), ["Mentions"]);
        assert_eq!(routes(&[row("deck", false, "Deck")], "cyex deck").unwrap(), ["Deck"]);
        assert_eq!(routes(&[row("deck", true, "Deck")], "cyex deck").unwrap(), ["Mentions", "Deck"]);
        assert_eq!(routes(&[row("deck", true, "")], "nothing here"), None);
    }

    #[test]
    fn a_row_going_nowhere_is_dropped_unless_it_pings() {
        assert_eq!(routes(&[row("deck", false, "")], "cyex deck"), None);
        let mut sound = row("deck", false, "");
        sound.alert.sound = true;
        assert_eq!(routes(&[sound], "cyex deck").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn two_rows_on_one_line_add_up_once() {
        let rows = [row("cyex", true, "Deck"), row("deck", true, "deck")];
        assert_eq!(routes(&rows, "cyex deck").unwrap(), ["Mentions", "Deck"]);
    }

    // V108-GM The row's tab choice lands in the global tab or the picked feed; own feeds stay.
    #[test]
    fn the_tab_choice_goes_where_global_mentions_go() {
        let r = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        assert_eq!(place(r(&["Mentions", "Deck"]), Some(TAB_FEED)), [TAB_FEED, "Deck"]);
        assert_eq!(place(r(&["Mentions", "Pings"]), Some("pings")), ["pings"]);
        assert_eq!(place(r(&["Mentions", "Deck"]), None), ["Deck"]);
        let mut h = Highlights { global_tab: false, ..Highlights::default() };
        assert_eq!(destination(&h), None);
        h.global_feed = " Pings ".into();
        assert_eq!(destination(&h).as_deref(), Some("Pings"));
        h.global_tab = true;
        assert_eq!(destination(&h).as_deref(), Some(TAB_FEED));
    }

    const LINE: &str = "@tmi-sent-ts=1792694804267;id=m1;room-id=1;user-id=2;display-name=Ann;badges=;color=;emotes= :ann!ann@ann.tmi.twitch.tv PRIVMSG #somechan :hey seryxx look";

    fn matched(raw: &str, own: &str, never: &str) -> Option<ChatMessage> {
        let set = MentionSet::new(&[row("seryxx", true, "")]);
        let peek = mentions::peek(raw)?;
        let (color, _) = outcome(&set, peek.text, &mut String::new())?;
        mention_of(raw, color, &mut Colors::default(), |id| id == own, |login| login == never)
    }

    // V111-12 A chat line matches like a firehose line; your own and never-highlight senders don't.
    #[test]
    fn v111_12_chat_lines_match_but_not_your_own() {
        let m = matched(LINE, "9", "").unwrap();
        assert_eq!(m.id, "m1");
        assert!(m.has(Flag::Mention));
        assert!(matched(LINE, "2", "").is_none(), "own message");
        assert!(matched(LINE, "9", "ann").is_none(), "never highlight");
        assert!(matched(&LINE.replace("seryxx", "someone"), "9", "").is_none());
    }

    // V111-12 The open chat's copy comes first; the firehose's copy of the same id is dropped.
    #[test]
    fn v111_12_the_later_copy_of_a_message_is_dropped() {
        let (mut gate, t) = (Gate::default(), Instant::now());
        assert!(gate.first("m1", t), "chat");
        assert!(!gate.first("m1", t + Duration::from_millis(900)), "firehose");
        assert!(gate.first("m2", t + Duration::from_secs(1)));
    }

    // V111-12 One alert gap for every path: a chat ping right after a firehose ping stays quiet.
    #[test]
    fn v111_12_one_alert_gap_for_every_path() {
        let (mut gate, t) = (Gate::default(), Instant::now());
        assert!(gate.alert(t), "firehose");
        assert!(!gate.alert(t + Duration::from_secs(1)), "chat, within the gap");
        assert!(gate.alert(t + ALERT_GAP), "after the gap");
    }
}
