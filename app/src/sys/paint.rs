//! V110-F01..F03 The Windows side of keeping windows painting (`windows::paint` decides).
//! WebView2's `ProcessFailed` per webview, the nudge that makes a webview take its surface
//! again, and the display, power and session notices the main window's subclass passes on.

/// What a webview's `ProcessFailed` said (V110-F01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The page's renderer crashed or was ended.
    RenderExited,
    /// The page's renderer stopped answering.
    Unresponsive,
    /// The GPU process went away (WebView2 starts a new one).
    GpuExited,
    /// Anything else (browser, utility, frame renderer…): logged only.
    Other(i32),
}

/// A display, power or session change the main window heard about (V110-F03), or the
/// start and end of a window drag (V110-F02).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    DisplayChanged,
    DpiChanged,
    Suspend,
    Resume,
    MonitorOff,
    MonitorOn,
    Lock,
    Unlock,
    SizeMoveStart,
    SizeMoveEnd,
}

type Sink = Box<dyn Fn(Notice) + Send + Sync>;

static NOTICES: std::sync::OnceLock<Sink> = std::sync::OnceLock::new();

/// Where the notices go. Set once.
pub fn on_notice(f: impl Fn(Notice) + Send + Sync + 'static) {
    let _ = NOTICES.set(Box::new(f));
}

fn notify(n: Notice) {
    if let Some(f) = NOTICES.get() {
        f(n);
    }
}

#[cfg(windows)]
mod imp {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED,
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED, COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE,
        COREWEBVIEW2_PROCESS_FAILED_REASON, ICoreWebView2ProcessFailedEventArgs2,
    };
    use webview2_com::ProcessFailedEventHandler;
    use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::System::Power::{POWERBROADCAST_SETTING, RegisterPowerSettingNotification};
    use windows::Win32::System::RemoteDesktop::{NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification};
    use windows::Win32::UI::WindowsAndMessaging::{
        DEVICE_NOTIFY_WINDOW_HANDLE, GetForegroundWindow, IsIconic, IsWindowVisible, PBT_APMRESUMEAUTOMATIC,
        PBT_APMSUSPEND, PBT_POWERSETTINGCHANGE, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
        WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
    };
    use windows::core::{GUID, Interface};

    use super::{Failure, Notice, notify};

    /// GUID_CONSOLE_DISPLAY_STATE (SystemServices, a feature this app doesn't build).
    /// Its data: 0 off, 1 on, 2 dimmed.
    const CONSOLE_DISPLAY_STATE: GUID = GUID::from_u128(0x6fe69556_704a_47a0_8f24_c28d936fda47);

    fn failure(kind: COREWEBVIEW2_PROCESS_FAILED_KIND) -> Failure {
        match kind {
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED => Failure::RenderExited,
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE => Failure::Unresponsive,
            COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED => Failure::GpuExited,
            other => Failure::Other(other.0),
        }
    }

    /// V110-F01 Calls `f` with each `ProcessFailed` of this webview, with the reason and exit
    /// code when the runtime has them (-1 when not). `f` runs on the UI thread.
    pub fn on_process_failed(webview: &tauri::Webview, f: impl Fn(Failure, i32, i32) + Send + 'static) {
        let _ = webview.with_webview(move |w| {
            // SAFETY: plain COM calls on the controller Tauri hands us, on the UI thread.
            let Ok(core) = (unsafe { w.controller().CoreWebView2() }) else {
                return;
            };
            let handler = ProcessFailedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
                // SAFETY: the args are live for this call; the out pointers are locals.
                unsafe { args.ProcessFailedKind(&mut kind)? };
                let (mut reason, mut code) = (COREWEBVIEW2_PROCESS_FAILED_REASON(-1), -1);
                if let Ok(args2) = args.cast::<ICoreWebView2ProcessFailedEventArgs2>() {
                    // SAFETY: as above.
                    unsafe {
                        let _ = args2.Reason(&mut reason);
                        let _ = args2.ExitCode(&mut code);
                    }
                }
                f(failure(kind), reason.0, code);
                Ok(())
            }));
            let mut token = 0i64;
            // SAFETY: WebView2 keeps the handler alive for as long as the webview lives.
            if let Err(e) = unsafe { core.add_ProcessFailed(&handler, &mut token) } {
                eprintln!("paint: process failed handler: {e}");
            }
        });
    }

    /// V110-F01 The nudge: the webview's bounds one px narrower and back, its parent's
    /// position again, and (unless the window is minimized or hidden) hidden and shown, so
    /// WebView2 takes a new surface and frame clock. Runs on the UI thread.
    pub fn nudge(webview: &tauri::Webview, done: impl FnOnce(Result<(), String>) + Send + 'static) {
        let sent = webview.with_webview(move |w| {
            let c = w.controller();
            // V110-F16 Every restoring call runs even when an earlier one failed, so a webview is
            // never left one px narrow or hidden; the first error is what comes back.
            let mut r: windows::core::Result<()> = Ok(());
            let mut keep = |x: windows::core::Result<()>| {
                if r.is_ok() {
                    r = x;
                }
            };
            // SAFETY: plain COM calls on the controller Tauri hands us and plain reads of its
            // parent window, all on the UI thread; out pointers are locals.
            unsafe {
                let mut hwnd = HWND::default();
                keep(c.ParentWindow(&mut hwnd));
                let mut b = RECT::default();
                keep(c.Bounds(&mut b));
                if b.right - b.left > 1 {
                    keep(c.SetBounds(RECT { right: b.right - 1, ..b }));
                    keep(c.SetBounds(b));
                }
                keep(c.NotifyParentWindowPositionChanged());
                if !hwnd.is_invalid() && IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() {
                    keep(c.SetIsVisible(false));
                    keep(c.SetIsVisible(true));
                }
            }
            done(r.map_err(|e| e.to_string()));
        });
        if let Err(e) = sent {
            eprintln!("paint: nudge not sent: {e}");
        }
    }

    /// Shown, not minimized: Windows' own state, readable from any thread.
    pub fn shown(window: &tauri::Window) -> bool {
        let Ok(hwnd) = window.hwnd() else { return false };
        // SAFETY: only reads the state of a live window.
        unsafe { IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() }
    }

    /// The window the user works in now.
    pub fn foreground(window: &tauri::Window) -> bool {
        let Ok(hwnd) = window.hwnd() else { return false };
        // SAFETY: takes nothing, returns a handle that is only compared.
        unsafe { GetForegroundWindow() == hwnd }
    }

    /// V110-F03 Asks Windows to tell the main window when the monitor turns on or off and
    /// when the session is locked or unlocked. Both end with the window.
    pub fn watch_display(window: &tauri::WebviewWindow) {
        let Ok(hwnd) = window.hwnd() else { return };
        // SAFETY: a live top-level window handle; the GUID outlives the call. The handles
        // returned live as long as the window, which is the app's.
        unsafe {
            if let Err(e) = RegisterPowerSettingNotification(
                HANDLE(hwnd.0),
                &CONSOLE_DISPLAY_STATE,
                DEVICE_NOTIFY_WINDOW_HANDLE,
            ) {
                eprintln!("paint: monitor notices: {e}");
            }
            if let Err(e) = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) {
                eprintln!("paint: session notices: {e}");
            }
        }
    }

    /// V110-F02/F03 Called by the window subclass (`frame`) for every message, before it
    /// goes on. Only reads; never answers a message.
    pub fn on_message(msg: u32, wparam: WPARAM, lparam: LPARAM) {
        let notice = match msg {
            WM_DISPLAYCHANGE => Some(Notice::DisplayChanged),
            WM_DPICHANGED => Some(Notice::DpiChanged),
            WM_ENTERSIZEMOVE => Some(Notice::SizeMoveStart),
            WM_EXITSIZEMOVE => Some(Notice::SizeMoveEnd),
            WM_WTSSESSION_CHANGE => match wparam.0 as u32 {
                WTS_SESSION_LOCK => Some(Notice::Lock),
                WTS_SESSION_UNLOCK => Some(Notice::Unlock),
                _ => None,
            },
            WM_POWERBROADCAST => match wparam.0 as u32 {
                PBT_APMSUSPEND => Some(Notice::Suspend),
                PBT_APMRESUMEAUTOMATIC => Some(Notice::Resume),
                PBT_POWERSETTINGCHANGE if lparam.0 != 0 => {
                    // SAFETY: for PBT_POWERSETTINGCHANGE, lparam points to a
                    // POWERBROADCAST_SETTING valid for this call; the data is read only
                    // when it holds at least one byte.
                    let s = unsafe { &*(lparam.0 as *const POWERBROADCAST_SETTING) };
                    (s.PowerSetting == CONSOLE_DISPLAY_STATE && s.DataLength >= 1).then(|| match s.Data[0] {
                        0 => Notice::MonitorOff,
                        _ => Notice::MonitorOn,
                    })
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(n) = notice {
            notify(n);
        }
    }
}

#[cfg(windows)]
pub use imp::{foreground, nudge, on_message, on_process_failed, shown, watch_display};

#[cfg(not(windows))]
pub fn on_process_failed(_webview: &tauri::Webview, _f: impl Fn(Failure, i32, i32) + Send + 'static) {}

#[cfg(not(windows))]
pub fn nudge(_webview: &tauri::Webview, done: impl FnOnce(Result<(), String>) + Send + 'static) {
    done(Ok(()));
}

#[cfg(not(windows))]
pub fn shown(window: &tauri::Window) -> bool {
    window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
}

#[cfg(not(windows))]
pub fn foreground(window: &tauri::Window) -> bool {
    window.is_focused().unwrap_or(false)
}

#[cfg(not(windows))]
pub fn watch_display(_window: &tauri::WebviewWindow) {}
