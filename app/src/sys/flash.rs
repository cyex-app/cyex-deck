//! V106-35 The taskbar button's flash. tao's `request_user_attention` blinks 4 times and
//! stops on its own, so the app calls `FlashWindowEx` itself: tray only (the custom frame
//! never blinks), until the window comes to the front or `stop` runs.

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        FLASHW_STOP, FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FLASHWINFO_FLAGS, FlashWindowEx,
    };

    fn call(window: &tauri::WebviewWindow, flags: FLASHWINFO_FLAGS, count: u32) {
        let Ok(hwnd) = window.hwnd() else { return };
        // A window handle can't cross threads; its number can.
        let raw = hwnd.0 as isize;
        let _ = window.run_on_main_thread(move || {
            let info = FLASHWINFO {
                cbSize: size_of::<FLASHWINFO>() as u32,
                hwnd: HWND(raw as _),
                dwFlags: flags,
                uCount: count,
                dwTimeout: 0,
            };
            // SAFETY: a live window handle and a FLASHWINFO with `cbSize` set. The return
            // value is the window's state before the call, not an error.
            let _ = unsafe { FlashWindowEx(&info) };
        });
    }

    /// V106-35 Flashes the taskbar button until the window comes to the front
    /// (FLASHW_TRAY | FLASHW_TIMERNOFG).
    pub fn start(window: &tauri::WebviewWindow) {
        call(window, FLASHW_TRAY | FLASHW_TIMERNOFG, u32::MAX);
    }

    /// Stops it (FLASHW_STOP).
    pub fn stop(window: &tauri::WebviewWindow) {
        call(window, FLASHW_STOP, 0);
    }
}

#[cfg(windows)]
pub use imp::{start, stop};

#[cfg(not(windows))]
pub fn start(_window: &tauri::WebviewWindow) {}

#[cfg(not(windows))]
pub fn stop(_window: &tauri::WebviewWindow) {}
