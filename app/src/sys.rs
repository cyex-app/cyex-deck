//! Windows calls that need `unsafe` (D28): which apps are running, for streamer mode
//! (U4), and WebView2's low-memory mode for windows nobody sees. The only module in the
//! app allowed to use `unsafe`; every call is small and checked here.
#![allow(unsafe_code)]

/// Lowercased exe names of the running processes. Empty if Windows won't say.
#[cfg(windows)]
pub fn running_exes() -> Vec<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };

    let mut names = Vec::new();
    // SAFETY: a process snapshot takes no pointers; the handle is closed below.
    let Ok(snap) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return names;
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: `entry` is a live PROCESSENTRY32W with `dwSize` set, as both calls require.
    let mut more = unsafe { Process32FirstW(snap, &mut entry) }.is_ok();
    while more {
        let len = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        names.push(String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase());
        // SAFETY: as above.
        more = unsafe { Process32NextW(snap, &mut entry) }.is_ok();
    }
    // SAFETY: `snap` came from CreateToolhelp32Snapshot and is closed once.
    let _ = unsafe { CloseHandle(snap) };
    names
}

#[cfg(not(windows))]
pub fn running_exes() -> Vec<String> {
    Vec::new()
}

/// Whether one of `apps` (exe names, any case) is running.
pub fn any_running(apps: &[String]) -> bool {
    if apps.is_empty() {
        return false;
    }
    let running = running_exes();
    apps.iter()
        .any(|a| running.iter().any(|r| r.eq_ignore_ascii_case(a.trim())))
}

/// Asks the window's WebView2 to give memory back while it's hidden or minimized, and to
/// go back to normal once it's shown. Older WebView2 runtimes without it are left alone.
#[cfg(windows)]
pub fn low_memory(window: &tauri::WebviewWindow, low: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        ICoreWebView2_19,
    };
    use windows::core::Interface;

    let level = if low {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
    } else {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
    };
    let _ = window.with_webview(move |w| {
        // SAFETY: plain COM calls on the controller Tauri hands us, on the UI thread.
        let set = unsafe { w.controller().CoreWebView2() }
            .and_then(|core| core.cast::<ICoreWebView2_19>())
            .and_then(|core| unsafe { core.SetMemoryUsageTargetLevel(level) });
        if let Err(e) = set {
            eprintln!("sys: memory level: {e}");
        }
    });
}

#[cfg(not(windows))]
pub fn low_memory(_window: &tauri::WebviewWindow, _low: bool) {}

/// V105-18 Windows 11's small rounded corners (4 px) for pop-ups. Windows 10 ignores it.
#[cfg(windows)]
pub fn small_corners(window: &tauri::WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL, DwmSetWindowAttribute};
    let Ok(hwnd) = window.hwnd() else { return };
    let pref = DWMWCP_ROUNDSMALL;
    // SAFETY: a live window handle and a 4-byte value of the attribute's type.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&pref).cast(),
            std::mem::size_of_val(&pref) as u32,
        )
    };
}

#[cfg(not(windows))]
pub fn small_corners(_window: &tauri::WebviewWindow) {}

/// V103-14 Fullscreen covers the whole monitor, also from a maximized window. tao keeps a
/// window maximized in fullscreen and gives a maximized frameless window only the monitor's
/// work area, so the taskbar's strip stayed empty next to the stream. This answers
/// WM_NCCALCSIZE first while the window is maximized and has no caption, which tao takes
/// away only for fullscreen: the client area is then the whole window, which tao sized to
/// the monitor. Every other message goes on to tao as before.
#[cfg(windows)]
pub fn fill_fullscreen(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetWindowLongW, IsZoomed, WM_NCCALCSIZE, WM_NCDESTROY, WS_CAPTION,
    };

    /// "cyex", next to Tauri's own subclass on the same window.
    const ID: usize = 0x6379_6578;

    unsafe extern "system" fn on_message(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        // SAFETY: Windows calls this with a live window on the thread that owns it; the
        // calls only read the window's state or pass the message on.
        unsafe {
            if msg == WM_NCCALCSIZE
                && wparam.0 != 0
                && IsZoomed(hwnd).as_bool()
                && (GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0) == 0
            {
                return LRESULT(0);
            }
            if msg == WM_NCDESTROY {
                let _ = RemoveWindowSubclass(hwnd, Some(on_message), ID);
            }
            DefSubclassProc(hwnd, msg, wparam, lparam)
        }
    }

    let Ok(hwnd) = window.hwnd() else { return };
    // A window handle can't cross threads; its number can.
    let raw = hwnd.0 as isize;
    let _ = window.run_on_main_thread(move || {
        // SAFETY: runs on the thread that owns the window, as subclassing requires. Setting
        // it again only updates the same subclass.
        if !unsafe { SetWindowSubclass(HWND(raw as _), Some(on_message), ID, 0) }.as_bool() {
            eprintln!("sys: fullscreen fill not set");
        }
    });
}

#[cfg(not(windows))]
pub fn fill_fullscreen(_window: &tauri::WebviewWindow) {}

/// V105-13 Font families installed on this PC (GDI), A-Z, without the vertical `@` ones.
#[cfg(windows)]
pub fn installed_fonts() -> Vec<String> {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::Graphics::Gdi::{
        DEFAULT_CHARSET, EnumFontFamiliesExW, GetDC, LOGFONTW, ReleaseDC, TEXTMETRICW,
    };

    unsafe extern "system" fn callback(lf: *const LOGFONTW, _: *const TEXTMETRICW, _: u32, lparam: LPARAM) -> i32 {
        // SAFETY: GDI passes a live LOGFONTW; `lparam` is the `&mut Vec<String>` given below.
        let (face, names) = unsafe { (&(*lf).lfFaceName, &mut *(lparam.0 as *mut Vec<String>)) };
        let len = face.iter().position(|&c| c == 0).unwrap_or(face.len());
        let name = String::from_utf16_lossy(&face[..len]);
        if !name.is_empty() && !name.starts_with('@') {
            names.push(name);
        }
        1
    }

    let mut names: Vec<String> = Vec::new();
    let query = LOGFONTW {
        lfCharSet: DEFAULT_CHARSET,
        ..Default::default()
    };
    // SAFETY: a screen DC is released below; `names` outlives the call and the callback
    // only runs inside it.
    unsafe {
        let hdc = GetDC(None);
        EnumFontFamiliesExW(hdc, &query, Some(callback), LPARAM(&mut names as *mut Vec<String> as isize), 0);
        ReleaseDC(None, hdc);
    }
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
}

#[cfg(not(windows))]
pub fn installed_fonts() -> Vec<String> {
    Vec::new()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn sees_itself_running() {
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().to_string();
        assert!(running_exes().contains(&name.to_lowercase()));
        assert!(any_running(&[name.to_uppercase()]));
        assert!(!any_running(&["no-such-app-here.exe".into()]));
        assert!(!any_running(&[]));
    }
}

/// V1051-02 Runs a script in a window's page and returns its result as JSON text, or `None`.
/// The result is never logged (it can hold a token).
#[cfg(windows)]
pub async fn run_script(window: &tauri::WebviewWindow, script: &'static str) -> Option<String> {
    use std::sync::Mutex;
    use webview2_com::ExecuteScriptCompletedHandler;
    use windows::core::HSTRING;

    let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
    let tx = Mutex::new(Some(tx));
    window
        .with_webview(move |w| {
            // SAFETY: plain COM calls on the controller Tauri hands us, on the UI thread.
            let core = unsafe { w.controller().CoreWebView2() };
            let handler = ExecuteScriptCompletedHandler::create(Box::new(move |res, json| {
                let tx = tx.lock().unwrap_or_else(|p| p.into_inner()).take();
                if let Some(tx) = tx {
                    let _ = tx.send(res.is_ok().then_some(json));
                }
                Ok(())
            }));
            if let Ok(core) = core {
                // SAFETY: the script string and handler outlive the call.
                let _ = unsafe { core.ExecuteScript(&HSTRING::from(script), &handler) };
            }
        })
        .ok()?;
    tokio::time::timeout(std::time::Duration::from_secs(2), rx)
        .await
        .ok()?
        .ok()?
}

#[cfg(not(windows))]
pub async fn run_script(_window: &tauri::WebviewWindow, _script: &'static str) -> Option<String> {
    None
}

/// V1051-03 Empties WebView2's disk cache (emotes, badges, paints). The app's windows share
/// one profile, so clearing it from any window clears it for all.
#[cfg(windows)]
pub async fn clear_picture_cache(window: &tauri::WebviewWindow) -> Result<(), String> {
    use std::sync::Mutex;
    use webview2_com::ClearBrowsingDataCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE, ICoreWebView2Profile2, ICoreWebView2_13,
    };
    use windows::core::Interface;

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let tx = Mutex::new(Some(tx));
    window
        .with_webview(move |w| {
            let finish = move |r: Result<(), String>| {
                if let Some(tx) = tx.lock().unwrap_or_else(|p| p.into_inner()).take() {
                    let _ = tx.send(r);
                }
            };
            // SAFETY: plain COM calls on the controller Tauri hands us, on the UI thread.
            let profile = unsafe { w.controller().CoreWebView2() }
                .and_then(|core| core.cast::<ICoreWebView2_13>())
                .and_then(|core| unsafe { core.Profile() })
                .and_then(|p| p.cast::<ICoreWebView2Profile2>());
            let profile = match profile {
                Ok(p) => p,
                Err(_) => return finish(Err("This WebView2 can’t clear its cache.".into())),
            };
            let done = std::sync::Arc::new(Mutex::new(Some(finish)));
            let on_done = done.clone();
            let handler = ClearBrowsingDataCompletedHandler::create(Box::new(move |res| {
                if let Some(f) = on_done.lock().unwrap_or_else(|p| p.into_inner()).take() {
                    f(res.map_err(|e| e.to_string()));
                }
                Ok(())
            }));
            // SAFETY: the handler outlives the call.
            let started = unsafe { profile.ClearBrowsingData(COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE, &handler) };
            if let Err(e) = started
                && let Some(f) = done.lock().unwrap_or_else(|p| p.into_inner()).take()
            {
                f(Err(e.to_string()));
            }
        })
        .map_err(|e| e.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(15), rx)
        .await
        .map_err(|_| "Clearing took too long.".to_string())?
        .map_err(|_| "Clearing didn’t finish.".to_string())?
}

#[cfg(not(windows))]
pub async fn clear_picture_cache(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}
