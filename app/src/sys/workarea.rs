//! V110-B10 Theatre is fullscreen, which covers the taskbar. At the bottom edge the window
//! shrinks to its monitor's work area (the monitor without the taskbar) so the taskbar shows,
//! and grows back to the whole monitor after. tao keeps its fullscreen state throughout, so
//! the app still counts as fullscreen.

/// A rectangle in physical px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// The rect to give the window: the work area while the taskbar should show, else the monitor.
/// A work area that isn't inside the monitor (or is empty) is not trusted: the monitor stays.
pub fn target(monitor: Rect, work: Rect, taskbar: bool) -> Rect {
    let inside = work.left >= monitor.left
        && work.top >= monitor.top
        && work.right <= monitor.right
        && work.bottom <= monitor.bottom
        && work.right > work.left
        && work.bottom > work.top;
    if taskbar && inside { work } else { monitor }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
    use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};

    use super::{Rect, target};

    fn rect(r: RECT) -> Rect {
        Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
    }

    /// Sizes `window` to its monitor's work area (`taskbar`) or the whole monitor.
    pub fn set(window: &tauri::WebviewWindow, taskbar: bool) {
        let Ok(hwnd) = window.hwnd() else { return };
        let raw = hwnd.0 as isize;
        // The window's thread does the resize, as the other window calls here do.
        let _ = window.run_on_main_thread(move || {
            let hwnd = HWND(raw as _);
            let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
            // SAFETY: reads the monitor of a live window into `info`, which is sized as the call requires.
            let ok = unsafe { GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info) }.as_bool();
            if !ok {
                return;
            }
            let r = target(rect(info.rcMonitor), rect(info.rcWork), taskbar);
            // SAFETY: moves a live window on its own thread; no activation, no z-order change.
            let _ = unsafe {
                SetWindowPos(hwnd, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOACTIVATE | SWP_NOZORDER)
            };
        });
    }
}

#[cfg(windows)]
pub use imp::set;

#[cfg(not(windows))]
pub fn set(_window: &tauri::WebviewWindow, _taskbar: bool) {}

#[cfg(test)]
mod tests {
    use super::*;

    const MON: Rect = Rect { left: 0, top: 0, right: 3840, bottom: 2160 };
    const WORK: Rect = Rect { left: 0, top: 0, right: 3840, bottom: 2112 };

    #[test]
    fn taskbar_shows_the_work_area() {
        assert_eq!(target(MON, WORK, true), WORK);
        assert_eq!(target(MON, WORK, false), MON);
    }

    #[test]
    fn a_work_area_outside_the_monitor_is_ignored() {
        let bad = Rect { left: 0, top: 0, right: 4000, bottom: 2112 };
        assert_eq!(target(MON, bad, true), MON);
        let empty = Rect { left: 0, top: 0, right: 0, bottom: 0 };
        assert_eq!(target(MON, empty, true), MON);
    }
}
