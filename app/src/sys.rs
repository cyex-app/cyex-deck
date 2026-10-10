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
