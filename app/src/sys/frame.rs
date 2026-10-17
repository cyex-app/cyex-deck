//! V103-14, V106-01 Our subclass on the main window and pop-outs: a maximized window in
//! fullscreen fills the whole screen, and the zoom's smallest size (`appearance.zoom`) is
//! answered here instead of through tao's `set_min_inner_size`, which re-applies the inner
//! size and restores a maximized window. The minimum lives in the subclass's own data, in
//! hundredths of a logical px, so it follows the window to a monitor with another scale.

/// Logical px in hundredths: a minimum survives the trip through the subclass data and back
/// to physical px exactly.
pub type Centi = (u32, u32);

/// Logical px to hundredths, rounded up.
pub fn centi((w, h): (f64, f64)) -> Centi {
    let c = |v: f64| (v * 100.0).ceil() as u32;
    (c(w), c(h))
}

/// The minimum in physical px at `dpi` (96 = 100 %), rounded up.
pub fn min_track((w, h): Centi, dpi: u32) -> (u32, u32) {
    let px = |v: u32| u32::try_from((u64::from(v) * u64::from(dpi)).div_ceil(9600)).unwrap_or(u32::MAX);
    (px(w), px(h))
}

/// Physical px at `dpi` back to hundredths, rounded down, so `min_track` at the same dpi
/// gives `physical` again and never more.
pub fn from_physical((w, h): (u32, u32), dpi: u32) -> Centi {
    let c = |v: u32| {
        let c = (u64::from(v) * 9600).checked_div(u64::from(dpi)).unwrap_or(0);
        u32::try_from(c).unwrap_or(u32::MAX)
    };
    (c(w), c(h))
}

/// The smallest whole window for a `client` minimum (physical px): the client minimum plus
/// what the window has around its client area now (`window` − `client` size, never less
/// than 0), so the client area itself stops at the minimum.
pub fn outer_min((w, h): (u32, u32), window: (i32, i32), client: (i32, i32)) -> (i32, i32) {
    let side = |min: u32, outer: i32, inner: i32| {
        i32::try_from(min)
            .unwrap_or(i32::MAX)
            .saturating_add(outer.saturating_sub(inner).max(0))
    };
    (side(w, window.0, client.0), side(h, window.1, client.1))
}

/// The subclass data: width in the high 32 bits, height in the low. 0 is no minimum.
fn pack((w, h): Centi) -> usize {
    usize::try_from((u64::from(w) << 32) | u64::from(h)).unwrap_or(0)
}

fn unpack(data: usize) -> Centi {
    let v = data as u64;
    ((v >> 32) as u32, v as u32)
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetClientRect, GetWindowLongW, GetWindowRect, IsIconic, IsZoomed, MINMAXINFO,
        WM_GETMINMAXINFO, WM_NCCALCSIZE, WM_NCDESTROY, WS_CAPTION,
    };

    use super::{Centi, min_track, outer_min, pack, unpack};

    /// Width and height of the window and of its client area now (physical px). Equal (no
    /// frame) when minimized or when either can't be read.
    fn sizes(hwnd: HWND) -> ((i32, i32), (i32, i32)) {
        let (mut outer, mut inner) = (RECT::default(), RECT::default());
        // SAFETY: only reads the rectangles of the live window this proc was called for.
        let ok = unsafe {
            !IsIconic(hwnd).as_bool()
                && GetWindowRect(hwnd, &mut outer).is_ok()
                && GetClientRect(hwnd, &mut inner).is_ok()
        };
        if !ok {
            return ((0, 0), (0, 0));
        }
        let size = |r: RECT| (r.right - r.left, r.bottom - r.top);
        (size(outer), size(inner))
    }

    // V106-01 user32's own export (Windows 10 1607+); the `windows` crate has it behind a
    // feature this app doesn't build.
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetDpiForWindow(hwnd: HWND) -> u32;
    }

    /// "cyex", next to Tauri's own subclass on the same window.
    const ID: usize = 0x6379_6578;

    /// V103-14 Fullscreen covers the whole monitor, also from a maximized window. tao keeps a
    /// window maximized in fullscreen and gives a maximized frameless window only the
    /// monitor's work area, so the taskbar's strip stayed empty next to the stream. This
    /// answers WM_NCCALCSIZE first while the window is maximized and has no caption, which
    /// tao takes away only for fullscreen: the client area is then the whole window, which
    /// tao sized to the monitor. V106-01 WM_GETMINMAXINFO gets the zoom minimum (`data`) on
    /// top of tao's. Every other message goes on to tao as before.
    unsafe extern "system" fn on_message(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        // SAFETY: Windows calls this with a live window on the thread that owns it; the
        // calls only read the window's state or pass the message on. For WM_GETMINMAXINFO,
        // a non-null `lparam` points to the MINMAXINFO Windows passes with this message,
        // valid and writable for the duration of the call and not aliased by anything else.
        unsafe {
            if msg == WM_NCCALCSIZE
                && wparam.0 != 0
                && IsZoomed(hwnd).as_bool()
                && (GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0) == 0
            {
                return LRESULT(0);
            }
            if msg == WM_GETMINMAXINFO && lparam.0 != 0 {
                // tao fills its own minimum first.
                let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                let min = min_track(unpack(data), GetDpiForWindow(hwnd));
                if min != (0, 0) {
                    // The minimum is for the client area; Windows applies it to the whole window.
                    let (window, client) = sizes(hwnd);
                    let (w, h) = outer_min(min, window, client);
                    let info = &mut *(lparam.0 as *mut MINMAXINFO);
                    info.ptMinTrackSize.x = info.ptMinTrackSize.x.max(w);
                    info.ptMinTrackSize.y = info.ptMinTrackSize.y.max(h);
                }
                return result;
            }
            if msg == WM_NCDESTROY {
                let _ = RemoveWindowSubclass(hwnd, Some(on_message), ID);
            }
            DefSubclassProc(hwnd, msg, wparam, lparam)
        }
    }

    /// Sets the subclass, or replaces its data on a window that has it.
    fn subclass(window: &tauri::WebviewWindow, data: usize) {
        let Ok(hwnd) = window.hwnd() else { return };
        // A window handle can't cross threads; its number can.
        let raw = hwnd.0 as isize;
        // The main thread runs these in order, so `attach` comes before the first minimum.
        let _ = window.run_on_main_thread(move || {
            // SAFETY: runs on the thread that owns the window, as subclassing requires.
            // Setting it again only updates the same subclass's data.
            if !unsafe { SetWindowSubclass(HWND(raw as _), Some(on_message), ID, data) }.as_bool() {
                eprintln!("sys: window subclass not set");
            }
        });
    }

    pub fn attach(window: &tauri::WebviewWindow) {
        subclass(window, 0);
    }

    pub fn set_min(window: &tauri::WebviewWindow, min: Centi) {
        subclass(window, pack(min));
    }
}

#[cfg(windows)]
pub use imp::{attach, set_min};

#[cfg(not(windows))]
pub fn attach(_window: &tauri::WebviewWindow) {}

#[cfg(not(windows))]
pub fn set_min(window: &tauri::WebviewWindow, (w, h): Centi) {
    let size = tauri::LogicalSize::new(f64::from(w) / 100.0, f64::from(h) / 100.0);
    let _ = window.set_min_size(Some(size));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_in_physical_px_rounds_up() {
        assert_eq!(min_track(centi((720.0, 420.0)), 96), (720, 420));
        assert_eq!(min_track(centi((720.0, 420.0)), 144), (1080, 630));
        assert_eq!(min_track(centi((720.5, 420.2)), 96), (721, 421));
        assert_eq!(min_track((0, 0), 144), (0, 0));
        assert_eq!(min_track(centi((720.0, 420.0)), 0), (0, 0));
    }

    #[test]
    fn physical_round_trip_never_grows() {
        for dpi in [96, 120, 144, 168, 192] {
            for px in [0, 1, 719, 1080, 1366, 2160, 3840] {
                let back = min_track(from_physical((px, px), dpi), dpi);
                assert_eq!(back, (px, px), "{px} px at {dpi} dpi");
            }
        }
        assert_eq!(from_physical((1080, 630), 0), (0, 0));
    }

    #[test]
    fn whole_window_minimum_adds_the_frame() {
        // 150 %: 11 px of invisible resize border each side, 1 px top and bottom.
        assert_eq!(outer_min((1080, 630), (2422, 1513), (2400, 1500)), (1102, 643));
        // Maximized in fullscreen, or minimized: no frame.
        assert_eq!(outer_min((1080, 630), (3840, 2160), (3840, 2160)), (1080, 630));
        // A client wider than its window (never expected) adds nothing.
        assert_eq!(outer_min((720, 420), (700, 400), (720, 420)), (720, 420));
        assert_eq!(outer_min((u32::MAX, 0), (10, 10), (0, 0)), (i32::MAX, 10));
    }

    #[test]
    fn data_packs_both_sides() {
        for c in [(0, 0), (72_000, 42_000), (u32::MAX, 1), (1, u32::MAX)] {
            assert_eq!(unpack(pack(c)), c);
        }
        assert_eq!(pack((0, 0)), 0);
    }
}
