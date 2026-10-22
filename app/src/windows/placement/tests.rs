use super::*;

const fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

const PRIMARY: Screen = Screen {
    bounds: rect(0, 0, 1920, 1080),
    work: rect(0, 0, 1920, 1032),
    scale: 1.0,
};
const RIGHT: Screen = Screen {
    bounds: rect(1920, -887, 3840, 2160),
    work: rect(1920, -887, 3840, 2088),
    scale: 1.5,
};
const LEFT: Screen = Screen {
    bounds: rect(-3840, -887, 3840, 2160),
    work: rect(-3840, -887, 3840, 2088),
    scale: 1.5,
};

/// The test machine: primary 1080p at 100 %, a 4K at 150 % right of it and one left.
fn screens() -> Vec<Screen> {
    vec![PRIMARY, RIGHT, LEFT]
}

fn geo(x: i32, y: i32, width: u32, height: u32, maximized: bool) -> Geometry {
    Geometry {
        x,
        y,
        width,
        height,
        maximized,
        scale: None,
    }
}

fn at(g: Geometry, scale: f64) -> Geometry {
    Geometry {
        scale: Some(scale),
        ..g
    }
}

const MIN: (f64, f64) = (720.0, 420.0);
const DEF: (f64, f64) = (1280.0, 800.0);

fn place3(g: &Geometry) -> Placement {
    place(g, &screens(), 0, MIN, DEF).unwrap()
}

/// Where it lands: the monitor (by bounds, so any order works) and the physical rect.
fn lands(p: &Placement, all: &[Screen]) -> (Rect, (i32, i32, i32, i32)) {
    (all[p.screen].bounds, (p.x, p.y, p.w, p.h))
}

#[test]
fn b1_a_save_on_the_right_4k_comes_back_there_in_any_order() {
    // Physical (1950, 0) 1200x900 on the right 4K saves as logical (1300, 0) 800x600 at 1.5.
    // At 1.0 that's on the primary too.
    let g = at(geo(1300, 0, 800, 600, false), 1.5);
    let mut orders = vec![screens(), vec![RIGHT, PRIMARY, LEFT], vec![LEFT, RIGHT, PRIMARY]];
    orders.push(screens().into_iter().rev().collect());
    for all in orders {
        let primary = all.iter().position(|s| *s == PRIMARY).unwrap();
        let p = place(&g, &all, primary, MIN, DEF).unwrap();
        assert_eq!(lands(&p, &all), (RIGHT.bounds, (1950, 0, 1200, 900)), "{all:?}");
        assert_eq!(p.scale, 1.5);
        assert_eq!(p.logical_size(), (800.0, 600.0));
        assert_eq!(p.logical_position(), (1300.0, 0.0));
    }
}

#[test]
fn b1_without_a_scale_the_old_guess_stays() {
    // The bug, kept for saves before v1.07: the primary claims it first.
    let p = place3(&geo(1300, 0, 800, 600, false));
    assert_eq!((p.screen, p.x, p.y, p.w, p.h), (0, 1120, 0, 800, 600));
}

#[test]
fn a_scaled_save_hanging_off_its_monitor_goes_by_overlap() {
    // Center past the right 4K's right edge (6150 > 5760): most of it is still on it.
    let p = place3(&at(geo(3700, -400, 800, 600, false), 1.5));
    assert_eq!((p.screen, p.x, p.y, p.w, p.h), (1, 5760 - 1200, -600, 1200, 900));
    // On no monitor at all: centered on the primary.
    let p = place3(&at(geo(9000, 9000, 800, 600, false), 1.5));
    assert_eq!((p.screen, p.x, p.y), (0, 560, 216));
}

#[test]
fn a_junk_scale_is_an_old_save() {
    for k in [0.0, -1.5, f64::NAN, 100.0] {
        let p = place3(&at(geo(1400, -400, 1280, 800, false), k));
        assert_eq!((p.screen, p.x, p.y), (1, 2100, -600), "{k}");
    }
}

#[test]
fn the_broken_save_lands_maximized_on_the_right_monitor() {
    // target/perf-v107/usertest/windows-broken.json: the maximized rect, 11 px outside.
    // V107-N12e Its normal rect is the default, centered there, so un-maximizing shrinks it.
    let p = place3(&geo(1273, -599, 2560, 1392, true));
    assert_eq!((p.screen, p.x, p.y, p.w, p.h), (1, 2880, -443, 1920, 1200));
    assert_eq!(p.logical_size(), (1280.0, 800.0));
    assert!(p.maximized);
    // Saved right away as what it got.
    assert_eq!(p.geometry(), at(geo(1920, -295, 1280, 800, true), 1.5));
    assert_eq!(place3(&p.geometry()), p, "and comes back the same");
}

#[test]
fn v107_n12e_only_a_maximized_full_rect_is_refit() {
    // Not maximized: a full-size window stays full size.
    let p = place3(&geo(0, 0, 1920, 1032, false));
    assert_eq!((p.x, p.y, p.w, p.h), (0, 0, 1920, 1032));
    // Maximized over a smaller normal rect: kept.
    let p = place3(&geo(100, 50, 1280, 800, true));
    assert_eq!((p.x, p.y, p.w, p.h, p.maximized), (100, 50, 1280, 800, true));
    // Maximized with one side short of the work area: kept.
    let p = place3(&geo(0, 0, 1920, 900, true));
    assert_eq!((p.w, p.h), (1920, 900));
}

#[test]
fn a_normal_rect_stays_where_it_was() {
    let p = place3(&geo(1400, -400, 1280, 800, false));
    assert_eq!((p.screen, p.x, p.y, p.w, p.h), (1, 2100, -600, 1920, 1200));
    let p = place3(&geo(100, 50, 1280, 800, false));
    assert_eq!((p.screen, p.x, p.y, p.w, p.h), (0, 100, 50, 1280, 800));
    let p = place3(&geo(-2000, -300, 1280, 800, false));
    assert_eq!((p.screen, p.x, p.y), (2, -3000, -450));
    // The same with their scales.
    let p = place3(&at(geo(1400, -400, 1280, 800, false), 1.5));
    assert_eq!((p.screen, p.x, p.y), (1, 2100, -600));
    let p = place3(&at(geo(-2000, -300, 1280, 800, false), 1.5));
    assert_eq!((p.screen, p.x, p.y), (2, -3000, -450));
}

#[test]
fn a_rect_hanging_off_an_edge_is_pulled_in() {
    // Mostly on the primary, bottom-right corner past it.
    let p = place3(&geo(1000, 600, 1280, 800, false));
    assert_eq!((p.screen, p.x, p.y), (0, 1920 - 1280, 1032 - 800));
    // Bigger than the primary: shrunk to its work area.
    let p = place3(&geo(0, 0, 2560, 1392, false));
    assert_eq!((p.screen, p.x, p.y, p.w, p.h), (0, 0, 0, 1920, 1032));
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
        assert_eq!((p.screen, p.x, p.y, p.w, p.h), (0, 320, 116, 1280, 800), "{g:?}");
        assert_eq!(p.maximized, g.maximized);
    }
}

#[test]
fn a_save_with_the_old_scale_finds_its_monitor() {
    // Saved mid-move from the primary: physical px of the left 4K at scale 1.
    let p = place3(&geo(-3700, -300, 1280, 800, false));
    assert_eq!((p.screen, p.x, p.y), (2, -3700, -300));
    assert_eq!(p.logical_size(), (1280.0, 800.0));
    // With the scale saved it's exact either way.
    let p = place3(&at(geo(-3700, -300, 1280, 800, false), 1.0));
    assert_eq!((p.screen, p.x, p.y), (2, -3700, -300));
}

#[test]
fn tiny_sizes_grow_to_the_minimum() {
    let p = place3(&geo(100, 100, 300, 200, false));
    assert_eq!((p.w, p.h), (720, 420));
    // At 150 % the minimum is still logical.
    let p = place3(&geo(1400, -400, 300, 200, false));
    assert_eq!((p.screen, p.w, p.h), (1, 1080, 630));
    assert_eq!(p.logical_size(), (720.0, 420.0));
}

#[test]
fn b5_a_work_area_smaller_than_the_minimum_keeps_the_minimum() {
    // 1000x600 px at 150 %: the 720x420 minimum is 1080x630 px, more than there is.
    let small = Screen {
        bounds: rect(1920, 0, 1000, 640),
        work: rect(1920, 0, 1000, 600),
        scale: 1.5,
    };
    let all = [PRIMARY, small];
    let p = place(&at(geo(1400, 100, 800, 500, false), 1.5), &all, 0, MIN, DEF).unwrap();
    // The size the OS gives it, placed from the work area's top-left so its caption shows.
    assert_eq!((p.screen, p.x, p.y, p.w, p.h), (1, 1920, 0, 1080, 630));
    // One axis short: the other is still clamped for the real size.
    let wide = Screen {
        bounds: rect(1920, 0, 2000, 640),
        work: rect(1920, 0, 2000, 600),
        scale: 1.5,
    };
    let all = [PRIMARY, wide];
    let p = place(&at(geo(2400, 100, 800, 500, false), 1.5), &all, 0, MIN, DEF).unwrap();
    assert_eq!((p.screen, p.x, p.y, p.w, p.h), (1, 1920 + 2000 - 1200, 0, 1200, 630));
    // Nothing saved: centered, or at the top-left where it's bigger.
    let p = place(&Geometry::default(), &all, 1, MIN, MIN).unwrap();
    assert_eq!((p.x, p.y, p.w, p.h), (1920 + 460, 0, 1080, 630));
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
    let g = centered_on(&RIGHT, DEF);
    assert_eq!(g, at(geo(1920, -295, 1280, 800, true), 1.5));
    let p = place3(&g);
    assert_eq!((p.screen, p.maximized), (1, true));
    assert_eq!((p.x, p.y), (1920 + 960, -887 + 444));
}

#[test]
fn b2_maximized_keeps_its_normal_rect_only_on_its_monitor() {
    let all = screens();
    let on_primary = at(geo(100, 50, 1280, 800, false), 1.0);
    // Still on the primary: the normal rect stays, flagged maximized.
    assert_eq!(
        maximized_on(Some(on_primary), &all, &PRIMARY, DEF),
        Geometry { maximized: true, ..on_primary }
    );
    // Moved maximized to the right 4K (Win+Shift+Arrow): centered there instead.
    let g = maximized_on(Some(on_primary), &all, &RIGHT, DEF);
    assert_eq!(g, centered_on(&RIGHT, DEF));
    let p = place3(&g);
    assert_eq!((p.screen, p.maximized), (1, true));
    // An old save without a scale goes by the guess; nothing or junk: centered.
    assert_eq!(
        maximized_on(Some(geo(1400, -400, 1280, 800, false)), &all, &RIGHT, DEF),
        geo(1400, -400, 1280, 800, true)
    );
    assert_eq!(maximized_on(None, &all, &LEFT, DEF), centered_on(&LEFT, DEF));
    assert_eq!(
        maximized_on(Some(geo(-32000, -32000, 160, 28, false)), &all, &LEFT, DEF),
        centered_on(&LEFT, DEF)
    );
}

#[test]
fn centering_fits_the_work_area() {
    assert_eq!(center(&PRIMARY, 1280.0, 800.0), (320, 116));
    assert_eq!(center(&RIGHT, 1920.0, 1200.0), (2880, -443));
    // Bigger than the work area: its top-left.
    assert_eq!(center(&PRIMARY, 4000.0, 2000.0), (0, 0));
}

#[test]
fn cards_open_beside_the_mouse_and_flip_near_the_edge() {
    // Room on the right: just after the mouse.
    assert_eq!(beside(100.0, 380.0, 12.0, 0.0, 1920.0), 112.0);
    // Near the right edge: left of the mouse, not over it.
    assert_eq!(beside(1800.0, 380.0, 12.0, 0.0, 1920.0), 1408.0);
    // Second monitor starting at 1920.
    assert_eq!(beside(1930.0, 380.0, 12.0, 1920.0, 1920.0), 1942.0);
    // Taller than either side: the roomier side, kept on screen.
    assert_eq!(beside(500.0, 900.0, 12.0, 0.0, 1040.0), 140.0);
    assert_eq!(beside(300.0, 900.0, 12.0, 0.0, 1040.0), 140.0);
    // Both axes on a monitor, physical px at 150 %.
    assert_eq!(beside_point(&RIGHT, (2000.0, 0.0), (570.0, 840.0), 18.0), (2018, 18));
    assert_eq!(beside_point(&RIGHT, (5700.0, 1150.0), (570.0, 840.0), 18.0), (5112, 292));
}
