//! Usercard windows (1.36): small pop-ups next to the mouse. Unpinned ones close when
//! they lose focus; pinned ones stay on top until closed. They share one saved size (V107:
//! the height too, fixed until then by V101-05); the messages scroll inside.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use cyex_settings::Partition;
use cyex_settings::schema::Windows;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl};

use super::panels::{PanelRegistry, label_for};
use crate::icons;
use super::placement::Rect;
use crate::settings::Settings;

const KIND: &str = "usercard";
/// V108-C09 Sent to a card that was open already when its user is clicked again: it
/// counts as opening it again (streamer mode blurs the picture again).
const REOPENED_EVENT: &str = "usercard:reopened";
const SIZE: (f64, f64) = (380.0, 680.0);
const WIDTHS: std::ops::RangeInclusive<f64> = 360.0..=640.0;
/// V107 Taller shows more of their messages. V111-18 The minimum fits the whole top part (header,
/// facts, roles with an open list capped at 18 % of the height, mod buttons) with about three
/// lines of logs left; the top part never scrolls, the logs give way first.
const HEIGHTS: std::ops::RangeInclusive<f64> = 680.0..=1400.0;
/// Focus can bounce between the window and its webview; only a lasting loss counts.
const BLUR_GRACE: Duration = Duration::from_millis(250);

/// Cards that have had focus. A card that never had it isn't closed for losing it,
/// e.g. while it's still being created.
fn focused_once() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(Mutex::default)
}

/// 1.36 An unpinned card closes when you click elsewhere.
pub fn on_focus(window: &tauri::Window, focused: bool) {
    let label = window.label().to_owned();
    let mut seen = focused_once().lock().unwrap_or_else(|p| p.into_inner());
    if focused {
        seen.insert(label);
        return;
    }
    if !seen.contains(&label) {
        return;
    }
    drop(seen);
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(BLUR_GRACE).await;
        if window.is_focused().unwrap_or(false) {
            return;
        }
        let app = window.app_handle();
        let open: Vec<(String, tauri::WebviewWindow)> = super::panels::open_panels_all(app)
            .into_iter()
            .filter(|(route, _)| is_card(route))
            .filter_map(|(route, label)| app.get_webview_window(&label).map(|w| (route, w)))
            .collect();
        let focused = open.iter().find(|(_, w)| w.is_focused().unwrap_or(false));
        if let Some((to, _)) = focused {
            // V109-C01d A usercard opened from a reply thread goes when you click back on the thread.
            let from = open.iter().find(|(_, w)| w.label() == window.label()).map(|(r, _)| r.as_str());
            if closes_on_return(from, to) && !window.is_always_on_top().unwrap_or(false) {
                focused_once().lock().unwrap_or_else(|p| p.into_inner()).remove(window.label());
                let _ = window.close();
            }
            // A card opened from this one (an @mention in its logs) stacks on it: both stay
            // while you move between them, and go together once you click elsewhere.
            return;
        }
        let cards: Vec<tauri::WebviewWindow> = open.into_iter().map(|(_, w)| w).collect();
        for w in cards {
            let seen = focused_once().lock().unwrap_or_else(|p| p.into_inner()).contains(w.label());
            if seen && !w.is_always_on_top().unwrap_or(false) {
                focused_once().lock().unwrap_or_else(|p| p.into_inner()).remove(w.label());
                let _ = w.close();
            }
        }
    });
}

/// V109-C01d A card left for a reply thread closes (it was opened from the thread); other
/// cards stack and stay while you move between them.
fn closes_on_return(from: Option<&str>, to: &str) -> bool {
    is_thread(to) && from.is_some_and(|f| !is_thread(f))
}

pub fn is_usercard(route: &str) -> bool {
    route == KIND || route.starts_with("usercard/")
}

/// V108-C27 The roles list of `/mods`, `/vips` and `/roles` (`roles/<channel>/<role>/<account>`).
fn is_roles(route: &str) -> bool {
    route.starts_with("roles/")
}

/// V109-C01 A reply thread (`thread/<channel>/<first message id>`).
fn is_thread(route: &str) -> bool {
    route.starts_with("thread/")
}

/// V108-C27 Cards: usercards, the roles list and V109-C01 reply threads. Unpinned they close when you click
/// elsewhere (a usercard opened from a list stacks on it), and layouts leave them out.
pub fn is_card(route: &str) -> bool {
    is_usercard(route) || is_roles(route) || is_thread(route)
}

/// V105-18 Pop-ups: usercards and `#channel` chats. Slim frame, small corners, and a
/// pin that's never remembered: every new one starts unpinned. Must match the `slim`
/// kinds in `ui/src/app/panels.ts`.
pub fn is_popup(route: &str) -> bool {
    is_card(route) || route.starts_with("room/")
}

/// Settings key for a route: all usercards share one entry, and so do the chats opened
/// from a `#channel` (V102-05), so `windows` doesn't grow with every channel.
pub fn settings_key(route: &str) -> &str {
    if is_usercard(route) {
        KIND
    } else if route.starts_with("room/") {
        "room"
    } else if is_roles(route) {
        "roles"
    } else if is_thread(route) {
        "thread"
    } else {
        route
    }
}

/// V111-18 The open cards of one user: the base label, then `-2`, `-3`... for extra ones.
fn cards_of(app: &AppHandle, base: &str) -> Vec<tauri::WebviewWindow> {
    (1..=20)
        .map(|n| if n == 1 { base.to_owned() } else { format!("{base}-{n}") })
        .filter_map(|l| app.get_webview_window(&l))
        .collect()
}

/// V111-18 First free label for another card of the same user.
fn free_label(app: &AppHandle, base: &str) -> String {
    (1..)
        .map(|n| if n == 1 { base.to_owned() } else { format!("{base}-{n}") })
        .find(|l| app.get_webview_window(l).is_none())
        .unwrap_or_else(|| base.to_owned())
}

pub fn open(app: &AppHandle, channel: &str, login: &str) -> tauri::Result<()> {
    let route = format!("{KIND}/{channel}/{login}");
    let base = label_for(&route);
    // V111-18 An unpinned card of this user is reused; when every open one is pinned, this
    // opens another beside them (a pinned card is one you keep).
    let open = cards_of(app, &base);
    if let Some(existing) = open.iter().find(|w| !w.is_always_on_top().unwrap_or(false)) {
        let _ = app.emit_to(existing.label(), REOPENED_EVENT, ());
        existing.show()?;
        return existing.set_focus();
    }
    let label = free_label(app, &base);
    let saved = app
        .state::<Settings>()
        .read::<Windows>(Partition::Windows)
        .panels
        .remove(KIND)
        .and_then(|p| p.geometry);
    let w = saved.map(|g| f64::from(g.width)).filter(|w| WIDTHS.contains(w)).unwrap_or(SIZE.0);
    let h = saved.map(|g| f64::from(g.height)).filter(|h| HEIGHTS.contains(h)).unwrap_or(SIZE.1);
    let url = WebviewUrl::App(format!("index.html#/panel/{route}").into());
    let window = super::builder(app, &label, url)
        // V102-14 The channel: the name is on the card already.
        .title(format!("#{channel}"))
        .inner_size(w, h)
        .min_inner_size(*WIDTHS.start(), *HEIGHTS.start())
        .max_inner_size(*WIDTHS.end(), *HEIGHTS.end())
        .skip_taskbar(true)
        .center()
        .build()?;
    crate::sys::small_corners(&window);
    // Placed in physical pixels while still hidden: the mouse may be on a monitor with
    // another scale than the one the window was created on, and Windows rescales it
    // when it moves there.
    let near = near_cursor(app, w, h).or_else(|| super::geometry::center_on_main(app, w, h));
    if let Some(p) = near {
        window.set_position(p)?;
    }
    // V111-19 Next to the pinned cards, never on them (moved only when the spot by the mouse
    // covers one).
    if let Some(p) = clear_of_pinned(app, w, h, near) {
        window.set_position(p)?;
    }
    app.state::<PanelRegistry>().insert(&label, &route);
    icons::apply(&window.as_ref().window());
    window.show()?;
    window.set_focus()
}

/// V111-19 Where the pinned usercards are (physical px), the last one first, and the work area of
/// the monitor of the first of them.
fn pinned_cards(app: &AppHandle) -> Vec<Rect> {
    let size = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
    let mut labels: Vec<_> = app.webview_windows().into_keys().filter(|l| l.starts_with("panel-usercard_")).collect();
    labels.sort_unstable_by(|a, b| b.cmp(a));
    labels
        .iter()
        .filter_map(|l| app.get_webview_window(l))
        .filter(|w| w.is_visible().unwrap_or(false) && w.is_always_on_top().unwrap_or(false))
        .filter_map(|w| {
            let (p, s) = w.outer_position().ok().zip(w.outer_size().ok())?;
            Some(Rect { x: p.x, y: p.y, w: size(s.width), h: size(s.height) })
        })
        .collect()
}

fn clear_of_pinned(app: &AppHandle, w: f64, h: f64, near: Option<PhysicalPosition<i32>>) -> Option<PhysicalPosition<i32>> {
    let pinned = pinned_cards(app);
    let first = pinned.first()?;
    let m = app.monitor_from_point(f64::from(first.x + first.w / 2), f64::from(first.y + first.h / 2)).ok().flatten()?;
    let s = super::geometry::screen(&m);
    // The saved size was in logical px; the cards are placed in physical ones.
    let size = ((w * s.scale).round() as i32, (h * s.scale).round() as i32);
    let gap = (8.0 * s.scale).round() as i32;
    if let Some(at) = near
        && pinned.iter().all(|q| overlap(q, (at.x, at.y), size) == 0)
    {
        return None;
    }
    let (x, y) = clear_spot(&s.work, &pinned, size, gap);
    Some(PhysicalPosition::new(x, y))
}

/// V111-19 Top-left corner for a `size` window that doesn't cover any of `pinned`: right of one
/// of them, else left, below, above (each tried per card, first card first), kept inside `work`.
/// When every spot overlaps something, the one that covers the least.
fn clear_spot(work: &Rect, pinned: &[Rect], (w, h): (i32, i32), gap: i32) -> (i32, i32) {
    let fit = |at: i32, size: i32, start: i32, room: i32| at.min(start + room - size).max(start);
    let mut best: Option<(i64, (i32, i32))> = None;
    for sides in 0..4 {
        for p in pinned {
            let (x, y) = match sides {
                0 => (p.x + p.w + gap, p.y),
                1 => (p.x - w - gap, p.y),
                2 => (p.x, p.y + p.h + gap),
                _ => (p.x, p.y - h - gap),
            };
            let at = (fit(x, w, work.x, work.w), fit(y, h, work.y, work.h));
            let covered: i64 = pinned.iter().map(|q| overlap(q, at, (w, h))).sum();
            if covered == 0 {
                return at;
            }
            if best.is_none_or(|(b, _)| covered < b) {
                best = Some((covered, at));
            }
        }
    }
    best.map_or((work.x, work.y), |(_, at)| at)
}

fn overlap(r: &Rect, (x, y): (i32, i32), (w, h): (i32, i32)) -> i64 {
    let dx = (r.x + r.w).min(x + w) - r.x.max(x);
    let dy = (r.y + r.h).min(y + h) - r.y.max(y);
    i64::from(dx.max(0)) * i64::from(dy.max(0))
}

/// Right of and below the mouse, or left of and above it where that side has no room, so
/// the card doesn't land on the name it's for; kept on its monitor and off the taskbar
/// (V101-04, V102-03). `w` and `h` are logical; the result is physical pixels. Also
/// places the chats opened from a `#channel` (V102-05).
pub fn near_cursor(app: &AppHandle, w: f64, h: f64) -> Option<PhysicalPosition<i32>> {
    let cursor = app.cursor_position().ok()?;
    // W1 Opened without the mouse on the app (keyboard, a test driver): not by the mouse.
    if !super::geometry::over_app(app, cursor.x, cursor.y) {
        return None;
    }
    let s = super::geometry::screen(&app.monitor_from_point(cursor.x, cursor.y).ok().flatten()?);
    let (x, y) = super::placement::beside_point(&s, (cursor.x, cursor.y), (w * s.scale, h * s.scale), 12.0 * s.scale);
    Some(PhysicalPosition::new(x, y))
}

#[cfg(test)]
mod tests {
    use super::{Rect, clear_spot, is_card, is_popup, settings_key};

    const WORK: Rect = Rect { x: 0, y: 0, w: 1920, h: 1040 };
    const SIZE: (i32, i32) = (380, 680);
    fn card(x: i32, y: i32) -> Rect {
        Rect { x, y, w: 380, h: 680 }
    }
    fn clear(at: (i32, i32), pinned: &[Rect]) -> bool {
        pinned.iter().all(|p| at.0 >= p.x + p.w || at.0 + SIZE.0 <= p.x || at.1 >= p.y + p.h || at.1 + SIZE.1 <= p.y)
    }

    #[test]
    fn new_card_opens_right_of_the_pinned_one() {
        let p = [card(100, 50)];
        assert_eq!(clear_spot(&WORK, &p, SIZE, 8), (488, 50));
    }

    #[test]
    fn left_when_right_is_off_screen() {
        let p = [card(1500, 50)];
        let at = clear_spot(&WORK, &p, SIZE, 8);
        assert_eq!(at, (1112, 50));
    }

    #[test]
    fn several_pinned_cards_are_all_avoided() {
        let p = [card(100, 50), card(488, 50), card(876, 50)];
        let at = clear_spot(&WORK, &p, SIZE, 8);
        assert!(clear(at, &p) && at.0 >= 0 && at.0 + SIZE.0 <= 1920, "{at:?}");
    }

    #[test]
    fn nothing_fits_covers_the_least() {
        // A work area just the size of one card: everything overlaps; the two offsets cover
        // the same, so any spot inside the work area is fine as long as it stays inside.
        let work = Rect { x: 0, y: 0, w: 400, h: 700 };
        let p = [card(0, 0)];
        let at = clear_spot(&work, &p, SIZE, 8);
        assert!(at.0 >= 0 && at.1 >= 0 && at.0 + SIZE.0 <= 400 && at.1 + SIZE.1 <= 700);
        // Two pinned cards, one narrower strip free: picks the side with less covered.
        let work = Rect { x: 0, y: 0, w: 800, h: 700 };
        let p = [card(0, 0), card(420, 0)];
        let at = clear_spot(&work, &p, (380, 680), 8);
        assert!(super::overlap(&p[0], at, SIZE) + super::overlap(&p[1], at, SIZE) <= 380 * 680);
    }

    #[test]
    fn popups_are_usercards_and_channel_chats() {
        assert!(is_popup("usercard/a/b"));
        assert!(is_popup("room/a"));
        // V108-C27 The roles list is a card too: one saved size, closes like a usercard.
        assert!(is_popup("roles/a/all/1") && is_card("roles/a/mod/") && !is_card("room/a"));
        assert_eq!(settings_key("roles/a/vip/1"), "roles");
        // V109-C01 A reply thread too.
        assert!(is_popup("thread/a/m1") && is_card("thread/a/m1"));
        assert_eq!(settings_key("thread/a/m1"), "thread");
        // V109-C01d Back on a thread, a usercard goes; between threads or usercards, both stay.
        assert!(super::closes_on_return(Some("usercard/a/b"), "thread/a/m1"));
        assert!(!super::closes_on_return(Some("thread/a/m2"), "thread/a/m1"));
        assert!(!super::closes_on_return(Some("usercard/a/b"), "usercard/a/c"));
        assert!(!is_popup("chat/x"));
        assert!(!is_popup("modview/a"));
        assert!(!is_popup("overlay/x"));
    }
}
