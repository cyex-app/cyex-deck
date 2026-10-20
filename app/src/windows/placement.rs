//! W1 Where a saved window comes back, as pure math over the connected monitors.
//!
//! Saved geometry is logical px at the scale it was saved at (`Geometry::scale`), so
//! `x * scale` is physical px. Saves from before v1.07 have no scale and are mapped back by
//! trying each monitor's own. The result is physical px, kept inside one monitor's work area
//! at a sane size, whatever was saved.

use cyex_settings::schema::Geometry;

/// Positions Windows gives minimized windows (-32000) and anything else no monitor has.
const FAR: i32 = 30_000;
/// Smallest and largest logical size worth restoring or saving.
pub const MIN_SANE: (u32, u32) = (200, 150);
const MAX_SANE: u32 = 16_384;
/// Scales a save can claim; anything else is junk and treated as an old save.
const SCALES: std::ops::RangeInclusive<f64> = 0.5..=8.0;

/// One monitor in physical px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub bounds: Rect,
    pub work: Rect,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= f64::from(self.x)
            && y >= f64::from(self.y)
            && x < f64::from(self.x) + f64::from(self.w)
            && y < f64::from(self.y) + f64::from(self.h)
    }

    fn overlap(&self, x: f64, y: f64, w: f64, h: f64) -> f64 {
        let dx = (f64::from(self.x + self.w).min(x + w) - f64::from(self.x).max(x)).max(0.0);
        let dy = (f64::from(self.y + self.h).min(y + h) - f64::from(self.y).max(y)).max(0.0);
        dx * dy
    }
}

/// Where and how big a window opens, in physical px, on `screens[screen]`. `scale` is that
/// monitor's, for the builder, which takes logical units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub screen: usize,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub scale: f64,
    pub maximized: bool,
}

impl Placement {
    pub fn logical_size(&self) -> (f64, f64) {
        (f64::from(self.w) / self.scale, f64::from(self.h) / self.scale)
    }

    pub fn logical_position(&self) -> (f64, f64) {
        (f64::from(self.x) / self.scale, f64::from(self.y) / self.scale)
    }
}

/// A rect worth saving or restoring: not a minimized window's, not a few px, not far away.
pub fn is_valid(g: &Geometry) -> bool {
    has_size(g) && g.x.abs() < FAR && g.y.abs() < FAR
}

fn has_size(g: &Geometry) -> bool {
    (MIN_SANE.0..=MAX_SANE).contains(&g.width) && (MIN_SANE.1..=MAX_SANE).contains(&g.height)
}

/// The monitor a saved rect belongs to, and the scale that maps its position there. `None`
/// when it's on none of them.
fn screen_of(g: &Geometry, screens: &[Screen]) -> Option<(usize, f64)> {
    match g.scale.filter(|k| SCALES.contains(k)) {
        Some(k) => saved_at(g, k, screens).map(|i| (i, k)),
        None => guess(g, screens),
    }
}

/// With its scale known, the rect is physical px: the monitor its center is on, else the
/// one it overlaps most. Monitors don't overlap in physical px, so there's one answer.
fn saved_at(g: &Geometry, k: f64, screens: &[Screen]) -> Option<usize> {
    let (x, y) = (f64::from(g.x) * k, f64::from(g.y) * k);
    let (w, h) = (f64::from(g.width) * k, f64::from(g.height) * k);
    screens
        .iter()
        .position(|s| s.bounds.contains(x + w / 2.0, y + h / 2.0))
        .or_else(|| most(screens, |_| (x, y, w, h)))
}

/// The monitor with the biggest share of a rect (physical px, per monitor).
fn most(screens: &[Screen], rect: impl Fn(&Screen) -> (f64, f64, f64, f64)) -> Option<usize> {
    screens
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (x, y, w, h) = rect(s);
            (i, s.bounds.overlap(x, y, w, h))
        })
        .filter(|&(_, a)| a > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// Saves without a scale: the monitor whose own scale puts the rect's center on it, else one
/// another monitor's scale does (a save racing a move between monitors used the old one),
/// else its top-left corner, else the one it overlaps most. Can pick the wrong one where
/// the logical ranges of monitors with different scales overlap.
fn guess(g: &Geometry, screens: &[Screen]) -> Option<(usize, f64)> {
    let (x, y) = (f64::from(g.x), f64::from(g.y));
    let (w, h) = (f64::from(g.width), f64::from(g.height));
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let own = |px: f64, py: f64| {
        screens
            .iter()
            .position(|s| s.bounds.contains(px * s.scale, py * s.scale))
            .map(|i| (i, screens[i].scale))
    };
    let any = || {
        screens.iter().find_map(|k| {
            screens
                .iter()
                .position(|s| s.bounds.contains(cx * k.scale, cy * k.scale))
                .map(|i| (i, k.scale))
        })
    };
    let most = || {
        most(screens, |s| (x * s.scale, y * s.scale, w * s.scale, h * s.scale)).map(|i| (i, screens[i].scale))
    };
    own(cx, cy).or_else(any).or_else(|| own(x, y)).or_else(most)
}

/// Top-left corner (physical px) that centers a `w` x `h` (physical) window in `s`'s work
/// area; a window bigger than it starts at its top-left.
pub fn center(s: &Screen, w: f64, h: f64) -> (i32, i32) {
    let at = |start: i32, room: i32, size: f64| {
        (f64::from(start) + ((f64::from(room) - size) / 2.0).max(0.0)).round() as i32
    };
    (at(s.work.x, s.work.w, w), at(s.work.y, s.work.h, h))
}

/// Top-left corner (physical px) of a `w` x `h` (physical) window opened beside the point
/// `(x, y)` on `s`: right of and below it, or left of and above it where that side has no
/// room, `gap` px away; inside the work area.
pub fn beside_point(s: &Screen, (x, y): (f64, f64), (w, h): (f64, f64), gap: f64) -> (i32, i32) {
    let x = beside(x, w, gap, f64::from(s.work.x), f64::from(s.work.w));
    let y = beside(y, h, gap, f64::from(s.work.y), f64::from(s.work.h));
    (x.round() as i32, y.round() as i32)
}

/// Along one axis: after `at` when the window fits there, else before it when it fits
/// there, else on the roomier side; always inside `start..start + len`.
fn beside(at: f64, size: f64, gap: f64, start: f64, len: f64) -> f64 {
    let end = start + len;
    let (after, before) = (at + gap, at - gap - size);
    let x = if after + size <= end || (before < start && end - at >= at - start) {
        after
    } else {
        before
    };
    x.min(end - size).max(start)
}

/// Places a saved window. A rect on no monitor or with junk values is centered on
/// `primary` at `default` size; the size is kept between `min` and the work area. `min`
/// wins over the work area, as the OS makes it: the position is clamped for that size.
/// `min` and `default` are logical. `None` when there are no monitors.
pub fn place(
    g: &Geometry,
    screens: &[Screen],
    primary: usize,
    min: (f64, f64),
    default: (f64, f64),
) -> Option<Placement> {
    let found = is_valid(g).then(|| screen_of(g, screens)).flatten();
    let screen = found
        .map(|(i, _)| i)
        .or_else(|| (primary < screens.len()).then_some(primary))
        .or_else(|| (!screens.is_empty()).then_some(0))?;
    let s = screens[screen];
    let (want_w, want_h) = if has_size(g) {
        (f64::from(g.width), f64::from(g.height))
    } else {
        default
    };
    // B5 The size the window really gets: never under `min`, even past the work area.
    let fit = |want: f64, min: f64, room: i32| {
        (want * s.scale)
            .min(f64::from(room))
            .max(min * s.scale)
            .round()
            .max(1.0)
    };
    let (pw, ph) = (fit(want_w, min.0, s.work.w), fit(want_h, min.1, s.work.h));
    let clamp = |at: f64, size: f64, start: i32, room: i32| {
        let end = f64::from(start) + f64::from(room) - size;
        at.round().min(end).max(f64::from(start)) as i32
    };
    let (x, y) = match found {
        Some((_, at)) => (
            clamp(f64::from(g.x) * at, pw, s.work.x, s.work.w),
            clamp(f64::from(g.y) * at, ph, s.work.y, s.work.h),
        ),
        None => center(&s, pw, ph),
    };
    Some(Placement {
        screen,
        x,
        y,
        w: pw as i32,
        h: ph as i32,
        scale: s.scale,
        maximized: g.maximized,
    })
}

/// A normal rect for a window that's maximized on `s` with no good normal rect saved:
/// `default` (logical) centered in the work area, so it comes back on that monitor.
pub fn centered_on(s: &Screen, default: (f64, f64)) -> Geometry {
    let w = (default.0 * s.scale).min(f64::from(s.work.w));
    let h = (default.1 * s.scale).min(f64::from(s.work.h));
    let (x, y) = center(s, w, h);
    Geometry {
        x: (f64::from(x) / s.scale).round() as i32,
        y: (f64::from(y) / s.scale).round() as i32,
        width: (w / s.scale).round() as u32,
        height: (h / s.scale).round() as u32,
        maximized: true,
        scale: Some(s.scale),
    }
}

/// B2 What a window maximized on `on` saves: the last normal rect while that's on `on`, so
/// un-maximizing lands where it was; else (moved there maximized, Win+Shift+Arrow) `normal`
/// centered on `on`, so it doesn't come back on the old monitor.
pub fn maximized_on(previous: Option<Geometry>, screens: &[Screen], on: &Screen, normal: (f64, f64)) -> Geometry {
    previous
        .filter(is_valid)
        .filter(|g| screen_of(g, screens).is_some_and(|(i, _)| screens[i].bounds == on.bounds))
        .map_or_else(|| centered_on(on, normal), |g| Geometry { maximized: true, ..g })
}

#[cfg(test)]
mod tests;
