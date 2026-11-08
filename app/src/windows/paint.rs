//! V110-F01..F03 Keeps windows painting. A window could stop drawing while its page still
//! ran (frozen frame, typing not shown) after a monitor, power or GPU change. Three ways
//! back, all ending in the nudge (`sys::paint::nudge`):
//! - F01 WebView2's `ProcessFailed`: a renderer that died or hangs reloads its page; a GPU
//!   process that went away nudges every window.
//! - F02 The main page pings once a second from an animation frame (`ui/src/lib/frameWatch.ts`).
//!   No ping for 3 s while the window is shown and the page still answers: nudge. Still
//!   none 3 s later: reload, at most once per 2 min.
//! - F03 Display, DPI, resume, monitor-on and unlock: every window is nudged after 500 ms.
//!
//! Each step goes to stderr and to `logs\paint.log` in the local data folder.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Webview, WebviewWindow};

use crate::sys::paint::{Failure, Notice};

/// No frame for this long while shown: the window may have stopped painting.
pub const STALL: Duration = Duration::from_secs(3);
/// After a nudge, frames get this long to come back before the page reloads.
pub const RETRY: Duration = Duration::from_secs(3);
/// At most one watchdog reload per window in this time.
pub const RELOAD_GAP: Duration = Duration::from_secs(120);
/// A hidden page in the foreground window is nudged at most this often (never reloaded).
const HIDDEN_GAP: Duration = Duration::from_secs(60);
/// A page that crashed or hung reloads at most this often.
const CRASH_GAP: Duration = Duration::from_secs(10);
/// A (re)loading page gets this long before its first ping counts as late.
const LOAD_GRACE: Duration = Duration::from_secs(10);
/// Display and power changes settle this long before the nudge.
const SETTLE: Duration = Duration::from_millis(500);
const TICK: Duration = Duration::from_secs(1);
/// A tick this late means the app didn't run (sleep, hibernate): every clock starts over.
const SUSPEND_GAP: Duration = Duration::from_secs(5);

/// What the watchdog does now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    None,
    Nudge,
    Reload,
}

/// What a page told the watchdog.
#[derive(Clone, Copy, Debug)]
pub struct Page {
    /// Its last frame ping, or when it was last given a fresh start (shown, loaded, dragged,
    /// resumed). May lie ahead (load grace).
    pub frame: Instant,
    /// `document.visibilityState` at its last ping.
    pub visible: bool,
    /// When it last answered a script.
    pub answer: Option<Instant>,
    /// When the watchdog last nudged it.
    pub nudged: Option<Instant>,
    /// It was visible when nudged. The nudge hides and shows the webview, so the page may
    /// say "hidden" right after; a stall that began visible still ends in a reload.
    pub nudged_visible: bool,
}

impl Page {
    fn fresh(at: Instant, visible: bool) -> Self {
        Self { frame: at, visible, answer: None, nudged: None, nudged_visible: false }
    }
}

/// What Windows says about the window and the system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gate {
    /// Minimized, hidden or in the tray.
    pub minimized: bool,
    /// Asleep, monitor off or session locked: nothing paints anyway.
    pub suspended: bool,
    /// A window drag or resize runs.
    pub dragging: bool,
    /// The window the user works in.
    pub foreground: bool,
}

/// The page has sent no frame for `STALL` where it should have: shown, the system awake,
/// no drag, and its page visible (or hidden although its window is in front).
pub fn stalled(p: &Page, g: &Gate, now: Instant) -> bool {
    !g.minimized
        && !g.suspended
        && !g.dragging
        && (p.visible || g.foreground)
        && now.saturating_duration_since(p.frame) >= STALL
}

/// V110-F02 The watchdog's step for one page. Nothing unless it stalled and answered a
/// script since the stall began (a page that doesn't answer is hung, F01's case). Then a
/// nudge; a visible page still without frames `RETRY` after it reloads. A hidden page is
/// only nudged, once per stall.
pub fn decide(p: &Page, g: &Gate, now: Instant) -> Step {
    if !stalled(p, g, now) {
        return Step::None;
    }
    let since = p.frame.checked_add(STALL).unwrap_or(p.frame);
    if !p.answer.is_some_and(|a| a >= since) {
        return Step::None;
    }
    match p.nudged.filter(|&n| n >= p.frame) {
        None => Step::Nudge,
        Some(n) if (p.visible || p.nudged_visible) && now.saturating_duration_since(n) >= RETRY => Step::Reload,
        Some(_) => Step::None,
    }
}

/// V110-F16 The step when the reload was refused: a nudge, but only once per `RELOAD_GAP`.
fn skipped_reload(nudges: &mut RateLimit, at: Instant) -> Step {
    if nudges.take(at) { Step::Nudge } else { Step::None }
}

/// At most one go per `gap`.
#[derive(Clone, Copy, Debug)]
pub struct RateLimit {
    gap: Duration,
    last: Option<Instant>,
}

impl RateLimit {
    pub const fn new(gap: Duration) -> Self {
        Self { gap, last: None }
    }

    /// Whether a go is allowed now; takes it when so.
    pub fn take(&mut self, now: Instant) -> bool {
        if self.last.is_some_and(|l| now.saturating_duration_since(l) < self.gap) {
            return false;
        }
        self.last = Some(now);
        true
    }
}

struct Entry {
    page: Page,
    /// Last tick's gate, to give a fresh start when the window comes back.
    gate: Gate,
    reloads: RateLimit,
    hidden_nudges: RateLimit,
    /// V110-F16 After a skipped reload: one more nudge per reload gap, not one per retry.
    skipped_nudges: RateLimit,
    /// The stall (its `frame`) already logged as unanswered.
    unanswered: Option<Instant>,
}

#[derive(Default)]
pub struct Paint {
    /// Pages that ping (the main window), by window label.
    pages: Mutex<HashMap<String, Entry>>,
    /// Webviews with the `ProcessFailed` handler.
    attached: Mutex<HashSet<String>>,
    crash_reloads: Mutex<HashMap<String, RateLimit>>,
    nudge_round: AtomicU64,
    asleep: AtomicBool,
    monitor_off: AtomicBool,
    locked: AtomicBool,
    dragging: AtomicBool,
    /// Windows reports the monitor's state once right after the sign-up: not a change.
    monitor_known: AtomicBool,
    #[cfg(feature = "mocks")]
    test: Mutex<TestState>,
}

#[cfg(feature = "mocks")]
#[derive(Default)]
struct TestState {
    /// Pings are dropped until then (a pretend freeze).
    frozen_until: Option<Instant>,
    nudges: u32,
    reloads: u32,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Sets up the state, the notices and the watchdog. Before any window exists.
pub fn init(app: &AppHandle) {
    app.manage(Paint::default());
    let notices = app.clone();
    crate::sys::paint::on_notice(move |n| on_notice(&notices, n));
    let ticks = app.clone();
    tauri::async_runtime::spawn(async move { watchdog(ticks).await });
}

/// V110-F03 The main window hears about the monitor and the session.
pub fn watch(main: &WebviewWindow) {
    crate::sys::paint::watch_display(main);
}

/// V110-F01 The `ProcessFailed` handler, once per webview.
pub fn attach(webview: &Webview) {
    let app = webview.app_handle().clone();
    let Some(state) = app.try_state::<Paint>() else { return };
    let label = webview.label().to_owned();
    if !lock(&state.attached).insert(label.clone()) {
        return;
    }
    let on = app.clone();
    crate::sys::paint::on_process_failed(webview, move |kind, reason, code| failed(&on, &label, kind, reason, code));
}

fn failed(app: &AppHandle, label: &str, kind: Failure, reason: i32, code: i32) {
    log(&format!("{label}: process failed: {kind:?} (reason {reason}, exit code {code})"));
    match kind {
        Failure::RenderExited | Failure::Unresponsive => {
            let Some(state) = app.try_state::<Paint>() else { return };
            let allowed = lock(&state.crash_reloads)
                .entry(label.to_owned())
                .or_insert(RateLimit::new(CRASH_GAP))
                .take(Instant::now());
            match app.get_webview_window(label) {
                Some(w) if allowed => {
                    log(&format!("{label}: reload"));
                    if let Err(e) = w.reload() {
                        log(&format!("{label}: reload failed: {e}"));
                    }
                }
                Some(_) => log(&format!("{label}: reload skipped (one ran under {} s ago)", CRASH_GAP.as_secs())),
                None => {}
            }
        }
        Failure::GpuExited => nudge_soon(app, "gpu process exited"),
        Failure::Other(_) => {}
    }
}

/// A page (re)loads: its pings count again after `LOAD_GRACE`.
pub fn page_started(app: &AppHandle, label: &str) {
    let Some(state) = app.try_state::<Paint>() else { return };
    if let Some(e) = lock(&state.pages).get_mut(label) {
        let at = Instant::now() + LOAD_GRACE;
        e.page = Page::fresh(at, e.page.visible);
    }
}

/// A window closed.
pub fn forget(app: &AppHandle, label: &str) {
    let Some(state) = app.try_state::<Paint>() else { return };
    lock(&state.attached).remove(label);
    lock(&state.pages).remove(label);
    lock(&state.crash_reloads).remove(label);
}

/// V110-F02 A frame ping from a page (`visible`), or the page going hidden.
pub fn frame(app: &AppHandle, label: &str, visible: bool) {
    let Some(state) = app.try_state::<Paint>() else { return };
    #[cfg(feature = "mocks")]
    if lock(&state.test).frozen_until.is_some_and(|t| Instant::now() < t) {
        return;
    }
    let now = Instant::now();
    let mut pages = lock(&state.pages);
    match pages.get_mut(label) {
        // A hidden notice isn't a frame.
        Some(e) if !visible => e.page.visible = false,
        Some(e) => {
            restart(&mut e.page, now);
            e.page.visible = true;
        }
        // A late ping for a window that closed doesn't bring its entry back.
        None if app.get_webview_window(label).is_none() => {}
        None => {
            pages.insert(
                label.to_owned(),
                Entry {
                    page: Page::fresh(now, visible),
                    gate: Gate::default(),
                    reloads: RateLimit::new(RELOAD_GAP),
                    hidden_nudges: RateLimit::new(HIDDEN_GAP),
                    skipped_nudges: RateLimit::new(RELOAD_GAP),
                    unanswered: None,
                },
            );
        }
    }
}

/// A fresh start for one page: its window moved, resized or came back.
pub fn touch(app: &AppHandle, label: &str) {
    let Some(state) = app.try_state::<Paint>() else { return };
    if let Some(e) = lock(&state.pages).get_mut(label) {
        restart(&mut e.page, Instant::now());
    }
}

fn touch_all(state: &Paint) {
    let now = Instant::now();
    for e in lock(&state.pages).values_mut() {
        restart(&mut e.page, now);
    }
}

fn restart(p: &mut Page, now: Instant) {
    if p.frame < now {
        p.frame = now;
    }
}

fn on_notice(app: &AppHandle, n: Notice) {
    let Some(state) = app.try_state::<Paint>() else { return };
    let set = |flag: &AtomicBool, on: bool| flag.store(on, Ordering::Relaxed);
    match n {
        Notice::SizeMoveStart => return set(&state.dragging, true),
        Notice::SizeMoveEnd => {
            set(&state.dragging, false);
            return touch_all(&state);
        }
        Notice::Suspend => set(&state.asleep, true),
        Notice::MonitorOff => set(&state.monitor_off, true),
        Notice::Lock => set(&state.locked, true),
        Notice::Resume => set(&state.asleep, false),
        Notice::MonitorOn => set(&state.monitor_off, false),
        Notice::Unlock => set(&state.locked, false),
        Notice::DisplayChanged | Notice::DpiChanged => {}
    }
    let first = matches!(n, Notice::MonitorOn | Notice::MonitorOff) && !state.monitor_known.swap(true, Ordering::Relaxed);
    touch_all(&state);
    if first {
        return;
    }
    match n {
        Notice::Suspend | Notice::MonitorOff | Notice::Lock => log(&format!("{n:?}: watchdog waits")),
        // The monitor-off notice comes again (dimmed, on) at times; one nudge per change is
        // enough, and `nudge_soon` folds those together.
        _ => nudge_soon(app, &format!("{n:?}")),
    }
}

/// V110-F03 Every window is nudged once things settled; a burst of changes gives one nudge.
pub fn nudge_soon(app: &AppHandle, why: &str) {
    let Some(state) = app.try_state::<Paint>() else { return };
    let round = state.nudge_round.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    let why = why.to_owned();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SETTLE).await;
        let Some(state) = app.try_state::<Paint>() else { return };
        if state.nudge_round.load(Ordering::Relaxed) == round {
            nudge_all(&app, &why);
        }
    });
}

/// The nudge on every window.
pub fn nudge_all(app: &AppHandle, why: &str) {
    log(&format!("nudge all windows ({why})"));
    for w in app.webview_windows().values() {
        nudge(w, why);
    }
    if let Some(state) = app.try_state::<Paint>() {
        touch_all(&state);
    }
}

fn nudge(w: &WebviewWindow, why: &str) {
    let label = w.label().to_owned();
    let why = why.to_owned();
    #[cfg(feature = "mocks")]
    if let Some(state) = w.try_state::<Paint>() {
        lock(&state.test).nudges += 1;
    }
    crate::sys::paint::nudge(w.as_ref(), move |r| {
        if let Err(e) = r {
            log(&format!("{label}: nudge ({why}) failed: {e}"));
        }
    });
}

async fn watchdog(app: AppHandle) {
    let mut last = Instant::now();
    loop {
        tokio::time::sleep(TICK).await;
        let now = Instant::now();
        let Some(state) = app.try_state::<Paint>() else { continue };
        if now.saturating_duration_since(last) >= SUSPEND_GAP {
            log(&format!(
                "tick came {} s late (asleep?): clocks start over",
                now.saturating_duration_since(last).as_secs()
            ));
            touch_all(&state);
        }
        last = now;
        check(&app, &state, now).await;
    }
}

fn gate(app: &AppHandle, state: &Paint, label: &str) -> Option<Gate> {
    let w = app.get_webview_window(label)?;
    let window = w.as_ref().window();
    let relaxed = |f: &AtomicBool| f.load(Ordering::Relaxed);
    Some(Gate {
        minimized: !crate::sys::paint::shown(&window),
        suspended: relaxed(&state.asleep) || relaxed(&state.monitor_off) || relaxed(&state.locked),
        dragging: relaxed(&state.dragging),
        foreground: crate::sys::paint::foreground(&window),
    })
}

/// One watchdog round: fresh gates, a script to each stalled page, then the steps.
async fn check(app: &AppHandle, state: &Paint, now: Instant) {
    let labels: Vec<String> = lock(&state.pages).keys().cloned().collect();
    for label in labels {
        let Some(g) = gate(app, state, &label) else {
            // The window is gone: drop what a late ping may have left.
            lock(&state.pages).remove(&label);
            continue;
        };
        let stalled = {
            let mut pages = lock(&state.pages);
            let Some(e) = pages.get_mut(&label) else { continue };
            // Shown again or brought to the front: a fresh start, not a stall.
            if (e.gate.minimized && !g.minimized) || (!e.gate.foreground && g.foreground && !e.page.visible) {
                restart(&mut e.page, now);
            }
            e.gate = g;
            stalled(&e.page, &g, now)
        };
        if !stalled {
            continue;
        }
        let Some(w) = app.get_webview_window(&label) else { continue };
        let answered = crate::sys::run_script(&w, "1").await.is_some();
        let at = Instant::now();
        let step = {
            let mut pages = lock(&state.pages);
            let Some(e) = pages.get_mut(&label) else { continue };
            if answered {
                e.page.answer = Some(at);
            }
            let quiet = at.saturating_duration_since(e.page.frame).as_millis();
            match decide(&e.page, &e.gate, at) {
                Step::None => {
                    if !answered && e.unanswered != Some(e.page.frame) {
                        // Once per stall: a page that doesn't answer is F01's (UNRESPONSIVE).
                        e.unanswered = Some(e.page.frame);
                        log(&format!("{label}: no frame for {quiet} ms and no answer: waiting for WebView2"));
                    }
                    Step::None
                }
                Step::Nudge if !e.page.visible && !e.hidden_nudges.take(at) => Step::None,
                Step::Nudge => {
                    e.page.nudged = Some(at);
                    e.page.nudged_visible = e.page.visible;
                    let seen = if e.page.visible { "visible" } else { "hidden in the front window" };
                    log(&format!("{label}: no frame for {quiet} ms, page answers ({seen}): nudge"));
                    Step::Nudge
                }
                Step::Reload if e.reloads.take(at) => {
                    log(&format!("{label}: still no frame after {quiet} ms: reload"));
                    e.page = Page::fresh(at + LOAD_GRACE, true);
                    Step::Reload
                }
                Step::Reload => {
                    // Looked at again after `RETRY`, but nudged once per reload gap.
                    e.page.nudged = Some(at);
                    let step = skipped_reload(&mut e.skipped_nudges, at);
                    if step == Step::Nudge {
                        log(&format!(
                            "{label}: still no frame, reload skipped (one ran under {} s ago): nudge again",
                            RELOAD_GAP.as_secs()
                        ));
                    }
                    step
                }
            }
        };
        match step {
            Step::None => {}
            Step::Nudge => nudge(&w, "no frames"),
            Step::Reload => {
                #[cfg(feature = "mocks")]
                {
                    lock(&state.test).reloads += 1;
                }
                if let Err(e) = w.reload() {
                    log(&format!("{label}: reload failed: {e}"));
                }
            }
        }
    }
}

/// Mocks only: `nudge` all windows now, `freeze` drops the main page's pings for 15 s (the
/// watchdog should nudge, then reload), `state` reads the counts.
#[cfg(feature = "mocks")]
pub fn dev(app: &AppHandle, action: &str) -> serde_json::Value {
    let Some(state) = app.try_state::<Paint>() else { return serde_json::Value::Null };
    match action {
        "nudge" => nudge_all(app, "dev"),
        "freeze" => lock(&state.test).frozen_until = Some(Instant::now() + Duration::from_secs(15)),
        _ => {}
    }
    let now = Instant::now();
    let pages: serde_json::Map<String, serde_json::Value> = lock(&state.pages)
        .iter()
        .map(|(l, e)| {
            let quiet = now.saturating_duration_since(e.page.frame).as_millis() as u64;
            (l.clone(), serde_json::json!({ "quietMs": quiet, "visible": e.page.visible, "gate": format!("{:?}", e.gate) }))
        })
        .collect();
    let t = lock(&state.test);
    serde_json::json!({ "nudges": t.nudges, "reloads": t.reloads, "pages": pages })
}

const LOG_CAP: u64 = 256 * 1024;

/// One line to stderr and `logs\paint.log` (the old half moves to `paint.old.log`).
fn log(msg: &str) {
    eprintln!("paint: {msg}");
    let Some(dir) = crate::crash::logs_dir() else { return };
    let path = dir.join("paint.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > LOG_CAP) {
        let _ = std::fs::rename(&path, dir.join("paint.old.log"));
    }
    let _ = std::fs::create_dir_all(dir);
    let stamp = stamp(time::OffsetDateTime::now_utc());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = writeln!(f, "{stamp} {msg}");
    }
}

/// `yyyy/mm/dd HH:MM:SS` UTC.
fn stamp(t: time::OffsetDateTime) -> String {
    format!(
        "{:04}/{:02}/{:02} {:02}:{:02}:{:02} UTC",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn a_skipped_reload_nudges_once_per_gap() {
        let t0 = Instant::now();
        let mut l = RateLimit::new(RELOAD_GAP);
        assert_eq!(skipped_reload(&mut l, t0), Step::Nudge);
        assert_eq!(skipped_reload(&mut l, t0 + secs(3)), Step::None);
        assert_eq!(skipped_reload(&mut l, t0 + secs(119)), Step::None);
        assert_eq!(skipped_reload(&mut l, t0 + secs(120)), Step::Nudge);
    }

    fn shown() -> Gate {
        Gate { foreground: true, ..Gate::default() }
    }

    /// A page whose last frame was at `t0`, answering `answer` s after it.
    fn page(t0: Instant, answer: Option<u64>, nudged: Option<u64>) -> Page {
        Page {
            frame: t0,
            visible: true,
            answer: answer.map(|a| t0 + secs(a)),
            nudged: nudged.map(|n| t0 + secs(n)),
            nudged_visible: nudged.is_some(),
        }
    }

    #[test]
    fn frames_coming_means_nothing() {
        let t0 = Instant::now();
        assert_eq!(decide(&page(t0, Some(2), None), &shown(), t0 + secs(2)), Step::None);
        assert_eq!(decide(&page(t0, Some(1), None), &shown(), t0 + Duration::from_millis(2999)), Step::None);
    }

    #[test]
    fn a_stall_with_an_answer_nudges_then_reloads() {
        let t0 = Instant::now();
        assert_eq!(decide(&page(t0, Some(3), None), &shown(), t0 + secs(3)), Step::Nudge);
        // Nudged at 3 s: nothing until 3 s later, then the reload.
        assert_eq!(decide(&page(t0, Some(5), Some(3)), &shown(), t0 + secs(5)), Step::None);
        assert_eq!(decide(&page(t0, Some(6), Some(3)), &shown(), t0 + secs(6)), Step::Reload);
    }

    #[test]
    fn no_answer_since_the_stall_is_not_ours() {
        let t0 = Instant::now();
        assert_eq!(decide(&page(t0, None, None), &shown(), t0 + secs(10)), Step::None);
        // An answer from before the stall doesn't count.
        assert_eq!(decide(&page(t0, Some(1), None), &shown(), t0 + secs(10)), Step::None);
    }

    #[test]
    fn minimized_asleep_or_dragged_waits() {
        let t0 = Instant::now();
        let p = page(t0, Some(9), None);
        let now = t0 + secs(10);
        for g in [
            Gate { minimized: true, ..shown() },
            Gate { suspended: true, ..shown() },
            Gate { dragging: true, ..shown() },
        ] {
            assert_eq!(decide(&p, &g, now), Step::None, "{g:?}");
        }
    }

    #[test]
    fn a_hidden_page_counts_only_in_the_front_window() {
        let t0 = Instant::now();
        let hidden = Page { visible: false, ..page(t0, Some(9), None) };
        assert!(!hidden.nudged_visible);
        let now = t0 + secs(10);
        // Covered by another window: Chromium hid it on purpose.
        assert_eq!(decide(&hidden, &Gate::default(), now), Step::None);
        assert_eq!(decide(&hidden, &shown(), now), Step::Nudge);
        let nudged = Page { nudged: Some(t0 + secs(4)), ..hidden };
        assert_eq!(decide(&nudged, &shown(), now), Step::None);
        // Visible when nudged, "hidden" after (the nudge hides and shows): still reloads.
        let flipped = Page { nudged_visible: true, ..nudged };
        assert_eq!(decide(&flipped, &shown(), now), Step::Reload);
    }

    #[test]
    fn a_frame_after_the_nudge_starts_a_new_stall() {
        let t0 = Instant::now();
        // Nudged at 3 s, frames back at 4 s, stalled again from 7 s.
        let p = Page { frame: t0 + secs(4), ..page(t0, Some(8), Some(3)) };
        assert_eq!(decide(&p, &shown(), t0 + secs(8)), Step::Nudge);
    }

    #[test]
    fn load_grace_lies_ahead() {
        let t0 = Instant::now();
        let p = Page::fresh(t0 + LOAD_GRACE, true);
        assert!(!stalled(&p, &shown(), t0 + secs(12)));
        assert!(stalled(&p, &shown(), t0 + LOAD_GRACE + STALL));
    }

    #[test]
    fn reloads_at_most_every_two_minutes() {
        let t0 = Instant::now();
        let mut r = RateLimit::new(RELOAD_GAP);
        assert!(r.take(t0));
        assert!(!r.take(t0));
        assert!(!r.take(t0 + RELOAD_GAP - secs(1)));
        assert!(r.take(t0 + RELOAD_GAP));
        assert!(!r.take(t0 + RELOAD_GAP + secs(1)));
    }

    #[test]
    fn stamps_read_like_the_app() {
        let t = time::macros::datetime!(2026-11-08 07:05:09 UTC);
        assert_eq!(stamp(t), "2026/11/08 07:05:09 UTC");
    }
}
