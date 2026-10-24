//! V108-03 The browser the user used last: the top-most visible browser window decides
//! which browser a sign-in opens in. Falls back to the default browser (the caller).

use std::path::PathBuf;

/// Exe names (lowercase, no `.exe`) of the browsers looked for.
const BROWSERS: [&str; 12] = [
    "chrome", "msedge", "firefox", "brave", "opera", "opera_gx", "vivaldi", "arc", "zen", "librewolf", "waterfox",
    "chromium",
];

/// One top-level window, in Z-order (top first).
#[derive(Clone, Debug)]
pub struct Win {
    pub pid: u32,
    pub exe: String,
    pub visible: bool,
    /// Tool windows (floating palettes) or cloaked ones (other desktops, hidden UWP frames).
    pub hidden_kind: bool,
}

fn is_browser(exe: &str) -> bool {
    let name = exe.to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    BROWSERS.contains(&name)
}

/// The pid of the top-most visible, normal browser window.
pub fn pick(windows: &[Win]) -> Option<u32> {
    windows
        .iter()
        .find(|w| w.visible && !w.hidden_kind && is_browser(&w.exe))
        .map(|w| w.pid)
}

/// Opens `url` in the browser used last. `false`: none found or it wouldn't start.
pub fn open_in_last_browser(url: &str) -> bool {
    let Some(exe) = pick(&win32::top_level()).and_then(win32::exe_path) else {
        return false;
    };
    launch(&exe, url)
}

/// The URL is one argument; no shell in between.
fn launch(exe: &PathBuf, url: &str) -> bool {
    std::process::Command::new(exe).arg(url).spawn().is_ok()
}

#[cfg(windows)]
mod win32 {
    use std::path::PathBuf;

    use ::windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, TRUE};
    use ::windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use ::windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, MODULEENTRY32W, Module32FirstW, TH32CS_SNAPMODULE,
    };
    use ::windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetWindowLongW, GetWindowThreadProcessId, IsWindowVisible, WS_EX_TOOLWINDOW,
    };
    use ::windows::core::BOOL;

    use super::Win;

    /// Top-level windows, top of the Z-order first (EnumWindows' order).
    pub fn top_level() -> Vec<Win> {
        let names: std::collections::HashMap<u32, String> =
            crate::sys::processes().into_iter().map(|(pid, _, exe)| (pid, exe)).collect();
        let mut found: Vec<(u32, bool, bool)> = Vec::new();
        // SAFETY: the callback only runs during the call and gets `found` back through
        // `lparam`, which outlives it.
        let _ = unsafe { EnumWindows(Some(each), LPARAM(&raw mut found as isize)) };
        found
            .into_iter()
            .map(|(pid, visible, hidden_kind)| Win {
                pid,
                exe: names.get(&pid).cloned().unwrap_or_default(),
                visible,
                hidden_kind,
            })
            .collect()
    }

    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the `found` vector `top_level` passed in, alive for the call.
        let found = unsafe { &mut *(lparam.0 as *mut Vec<(u32, bool, bool)>) };
        let mut pid = 0u32;
        // SAFETY: plain window queries on a handle EnumWindows gave us.
        let visible = unsafe {
            GetWindowThreadProcessId(hwnd, Some(&raw mut pid));
            IsWindowVisible(hwnd).as_bool()
        };
        // SAFETY: as above.
        let tool = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOOLWINDOW.0 != 0;
        let mut cloaked = 0u32;
        // SAFETY: `cloaked` is a u32, the size DWMWA_CLOAKED writes.
        let _ = unsafe {
            DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, (&raw mut cloaked).cast(), size_of::<u32>() as u32)
        };
        found.push((pid, visible, tool || cloaked != 0));
        TRUE
    }

    /// The full path of the process's exe (its first module).
    pub fn exe_path(pid: u32) -> Option<PathBuf> {
        // SAFETY: a module snapshot takes no pointers; the handle is closed below.
        let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, pid) }.ok()?;
        let mut entry = MODULEENTRY32W {
            dwSize: size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };
        // SAFETY: `entry` is a live MODULEENTRY32W with `dwSize` set.
        let ok = unsafe { Module32FirstW(snap, &mut entry) }.is_ok();
        // SAFETY: `snap` came from CreateToolhelp32Snapshot and is closed once.
        let _ = unsafe { CloseHandle(snap) };
        if !ok {
            return None;
        }
        let len = entry.szExePath.iter().position(|&c| c == 0).unwrap_or(entry.szExePath.len());
        let path = PathBuf::from(String::from_utf16_lossy(&entry.szExePath[..len]));
        (path.is_file() && super::is_browser(&path.file_name()?.to_string_lossy())).then_some(path)
    }
}

#[cfg(not(windows))]
mod win32 {
    pub fn top_level() -> Vec<super::Win> {
        Vec::new()
    }
    pub fn exe_path(_pid: u32) -> Option<std::path::PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(pid: u32, exe: &str, visible: bool, hidden_kind: bool) -> Win {
        Win { pid, exe: exe.into(), visible, hidden_kind }
    }

    #[test]
    fn top_most_visible_browser_wins() {
        let list = [
            w(1, "cyex-deck.exe", true, false),
            w(2, "Code.exe", true, false),
            w(3, "firefox.exe", false, false),
            w(4, "chrome.exe", true, true),
            w(5, "msedge.exe", true, false),
            w(6, "chrome.exe", true, false),
        ];
        assert_eq!(pick(&list), Some(5));
    }

    #[test]
    fn names_match_without_case_or_exe() {
        assert!(is_browser("Opera_GX.EXE"));
        assert!(is_browser("zen"));
        assert!(!is_browser("chromedriver.exe"));
        assert!(!is_browser("explorer.exe"));
        assert!(!is_browser(""));
    }

    #[test]
    fn nothing_found_falls_back() {
        assert_eq!(pick(&[]), None);
        assert_eq!(pick(&[w(1, "notepad.exe", true, false), w(2, "brave.exe", false, false)]), None);
    }
}
