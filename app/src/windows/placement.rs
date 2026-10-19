//! W1 Where a saved window comes back, as pure math over the connected monitors.
//!
//! Saved geometry is logical px at the scale of the monitor the window was on, so a point is
//! mapped back by trying each monitor's own scale. The result is physical px, kept inside one
//! monitor's work area at a sane size, whatever was saved.

use cyex_settings::schema::Geometry;

/// Positions Windows gives minimized windows (-32000) and anything else no monitor has.
const FAR: i32 = 30_000;
/// Smallest and largest logical size worth restoring or saving.
pub const MIN_SANE: (u32, u32) = (200, 150);
const MAX_SANE: u32 = 16_384;

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
    fn contains(&self, x: f64, y: f64) -> bool {
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

/// Where and how big a window opens: physical position of its top-left corner, logical
/// size, on `screens[screen]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub screen: usize,
    pub x: i32,
    pub y: i32,
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}

/// A rect worth saving or restoring: not a minimized window's, not a few px, not far away.
pub fn is_valid(g: &Geometry) -> bool {
    has_size(g) && g.x.abs() < FAR && g.y.abs() < FAR
}

fn has_size(g: &Geometry) -> bool {
    (MIN_SANE.0..=MAX_SANE).contains(&g.width) && (MIN_SANE.1..=MAX_SANE).contains(&g.height)
}

/// The monitor a saved rect belongs to, and the scale that maps its position there: the one
/// whose own scale puts the rect's center on it, else one another monitor's scale does (a
/// save racing a move between monitors used the old one), else its top-left corner, else the
/// one it overlaps most. `None` when it's on none of them.
fn screen_of(g: &Geometry, screens: &[Screen]) -> Option<(usize, f64)> {
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
        screens
            .iter()
            .enumerate()
            .map(|(i, s)| {
                (
                    i,
                    s.bounds
                        .overlap(x * s.scale, y * s.scale, w * s.scale, h * s.scale),
                )
            })
            .filter(|&(_, a)| a > 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| (i, screens[i].scale))
    };
    own(cx, cy).or_else(any).or_else(|| own(x, y)).or_else(most)
}

/// Places a saved window. A rect on no monitor or with junk values is centered on the
/// primary monitor at `default` size; the size is kept between `min` and the work area.
/// `min` and `default` are logical. `None` when there are no monitors.
pub fn place(
    g: &Geometry,
    screens: &[Screen],
    primary: usize,
    min: (f64, f64),
    default: (f64, f64),
) -> Option<Placement> {
    let found = is_valid(g).then(|| screen_of(g, screens)).flatten();
    let at = found.map_or(1.0, |(_, k)| k);
    let found = found.map(|(i, _)| i);
    let screen = found
        .or_else(|| (primary < screens.len()).then_some(primary))
        .or_else(|| (!screens.is_empty()).then_some(0))?;
    let s = screens[screen];
    let (want_w, want_h) = if has_size(g) {
        (f64::from(g.width), f64::from(g.height))
    } else {
        default
    };
    let fit = |want: f64, min: f64, room: i32| {
        (want * s.scale)
            .max(min * s.scale)
            .min(f64::from(room))
            .round()
            .max(1.0)
    };
    let (pw, ph) = (fit(want_w, min.0, s.work.w), fit(want_h, min.1, s.work.h));
    let clamp = |at: f64, size: f64, start: i32, room: i32| {
        let end = f64::from(start) + f64::from(room) - size;
        at.round().min(end).max(f64::from(start)) as i32
    };
    let (x, y) = match found {
        Some(_) => (
            clamp(f64::from(g.x) * at, pw, s.work.x, s.work.w),
            clamp(f64::from(g.y) * at, ph, s.work.y, s.work.h),
        ),
        None => (
            s.work.x + ((f64::from(s.work.w) - pw) / 2.0).round() as i32,
            s.work.y + ((f64::from(s.work.h) - ph) / 2.0).round() as i32,
        ),
    };
    Some(Placement {
        screen,
        x,
        y,
        width: pw / s.scale,
        height: ph / s.scale,
        maximized: g.maximized,
    })
}

/// A normal rect for a window that's maximized on `s` with no good normal rect saved:
/// `default` (logical) centered in the work area, so it comes back on that monitor.
pub fn centered_on(s: &Screen, default: (f64, f64)) -> Geometry {
    let w = (default.0 * s.scale).min(f64::from(s.work.w));
    let h = (default.1 * s.scale).min(f64::from(s.work.h));
    let x = f64::from(s.work.x) + (f64::from(s.work.w) - w) / 2.0;
    let y = f64::from(s.work.y) + (f64::from(s.work.h) - h) / 2.0;
    Geometry {
        x: (x / s.scale).round() as i32,
        y: (y / s.scale).round() as i32,
        width: (w / s.scale).round() as u32,
        height: (h / s.scale).round() as u32,
        maximized: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The test machine: primary 1080p at 100 %, a 4K at 150 % right of it and one left.
    fn screens() -> Vec<Screen> {
        vec![
            Screen {
                bounds: rect(0, 0, 1920, 1080),
                work: rect(0, 0, 1920, 1032),
                scale: 1.0,
            },
            Screen {
                bounds: rect(1920, -887, 3840, 2160),
                work: rect(1920, -887, 3840, 2088),
                scale: 1.5,
            },
            Screen {
                bounds: rect(-3840, -887, 3840, 2160),
                work: rect(-3840, -887, 3840, 2088),
                scale: 1.5,
            },
        ]
    }

    fn geo(x: i32, y: i32, width: u32, height: u32, maximized: bool) -> Geometry {
        Geometry {
            x,
            y,
            width,
            height,
            maximized,
        }
    }

    const MIN: (f64, f64) = (720.0, 420.0);
    const DEF: (f64, f64) = (1280.0, 800.0);

    fn place3(g: &Geometry) -> Placement {
        place(g, &screens(), 0, MIN, DEF).unwrap()
    }

    #[test]
    fn the_broken_save_lands_maximized_on_the_right_monitor() {
        // target/perf-v107/usertest/windows-broken.json: the maximized rect, 11 px outside.
        let p = place3(&geo(1273, -599, 2560, 1392, true));
        assert_eq!(p.screen, 1);
        assert_eq!((p.x, p.y), (1920, -887));
        assert_eq!((p.width, p.height), (2560.0, 1392.0));
        assert!(p.maximized);
    }

    #[test]
    fn a_normal_rect_stays_where_it_was() {
        let p = place3(&geo(1400, -400, 1280, 800, false));
        assert_eq!((p.screen, p.x, p.y), (1, 2100, -600));
        assert_eq!((p.width, p.height), (1280.0, 800.0));
        let p = place3(&geo(100, 50, 1280, 800, false));
        assert_eq!((p.screen, p.x, p.y), (0, 100, 50));
        let p = place3(&geo(-2000, -300, 1280, 800, false));
        assert_eq!((p.screen, p.x, p.y), (2, -3000, -450));
    }

    #[test]
    fn a_rect_hanging_off_an_edge_is_pulled_in() {
        // Mostly on the primary, bottom-right corner past it.
        let p = place3(&geo(1000, 600, 1280, 800, false));
        assert_eq!(p.screen, 0);
        assert_eq!((p.x, p.y), (1920 - 1280, 1032 - 800));
        // Bigger than the primary: shrunk to its work area.
        let p = place3(&geo(0, 0, 2560, 1392, false));
        assert_eq!((p.screen, p.x, p.y), (0, 0, 0));
        assert_eq!((p.width, p.height), (1920.0, 1032.0));
    }

    #[test]
    fn a_gone_monitor_or_junk_centers_on_the_primary() {
        for g in [
            geo(9000, 9000, 1280, 800, false),   // monitor gone
            geo(-32000, -32000, 160, 28, false), // minimized
            geo(-22613, -21629, 107, 19, true),  // the broken window
            geo(0, 0, 0, 0, true),               // nothing saved but the flag
        ] {
            let p = place3(&g);
            assert_eq!(p.screen, 0, "{g:?}");
            assert_eq!((p.width, p.height), (1280.0, 800.0), "{g:?}");
            assert_eq!((p.x, p.y), (320, 116), "{g:?}");
            assert_eq!(p.maximized, g.maximized);
        }
    }

    #[test]
    fn a_save_with_the_old_scale_finds_its_monitor() {
        // Saved mid-move from the primary: physical px of the left 4K at scale 1.
        let p = place3(&geo(-3700, -300, 1280, 800, false));
        assert_eq!((p.screen, p.x, p.y), (2, -3700, -300));
        assert_eq!((p.width, p.height), (1280.0, 800.0));
    }

    #[test]
    fn tiny_sizes_grow_to_the_minimum() {
        let p = place3(&geo(100, 100, 300, 200, false));
        assert_eq!((p.width, p.height), (720.0, 420.0));
        // At 150 % the minimum is still logical.
        let p = place3(&geo(1400, -400, 300, 200, false));
        assert_eq!((p.screen, p.width, p.height), (1, 720.0, 420.0));
    }

    #[test]
    fn no_monitors_means_no_placement() {
        assert_eq!(place(&geo(0, 0, 800, 600, false), &[], 0, MIN, DEF), None);
        // A primary index past the list falls back to the first monitor.
        let p = place(&geo(9000, 9000, 800, 600, false), &screens(), 7, MIN, DEF).unwrap();
        assert_eq!(p.screen, 0);
    }

    #[test]
    fn only_real_rects_are_saved() {
        assert!(is_valid(&geo(1273, -599, 2560, 1392, true)));
        assert!(!is_valid(&geo(-32000, -32000, 1280, 800, false)));
        assert!(!is_valid(&geo(100, 100, 107, 19, false)));
        assert!(!is_valid(&geo(100, 100, 1280, 0, false)));
        assert!(!is_valid(&geo(100, 100, 100_000, 800, false)));
    }

    #[test]
    fn maximized_without_a_normal_rect_keeps_its_monitor() {
        let g = centered_on(&screens()[1], DEF);
        assert_eq!(g, geo(1920, -295, 1280, 800, true));
        let p = place3(&g);
        assert_eq!((p.screen, p.maximized), (1, true));
        assert_eq!((p.x, p.y), (1920 + 960, -887 + 444));
    }
}
