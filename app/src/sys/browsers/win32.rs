//! V108-03 The Windows side: top-level windows in Z-order, a process's exe and command
//! line, and the browsers Windows lists as installed.

use std::path::PathBuf;

use ::windows::Wdk::System::Threading::{NtQueryInformationProcess, ProcessCommandLineInformation};
use ::windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, HWND, LPARAM, TRUE, UNICODE_STRING};
use ::windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use ::windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW,
    RegGetValueW, RegOpenKeyExW,
};
use ::windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetWindowLongW, GetWindowThreadProcessId, IsWindowVisible, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};
use ::windows::core::{BOOL, HSTRING, PCWSTR, PWSTR};

use super::Win;

/// Top-level windows, top of the Z-order first (EnumWindows' order).
pub fn top_level() -> Vec<Win> {
    let names: std::collections::HashMap<u32, String> = crate::sys::processes()
        .into_iter()
        .map(|(pid, _, exe)| (pid, exe))
        .collect();
    let mut found: Vec<(u32, bool, bool, bool)> = Vec::new();
    // SAFETY: the callback only runs during the call and gets `found` back through
    // `lparam`, which outlives it.
    let _ = unsafe { EnumWindows(Some(each), LPARAM(&raw mut found as isize)) };
    found
        .into_iter()
        .map(|(pid, visible, hidden_kind, topmost)| Win {
            pid,
            exe: names.get(&pid).cloned().unwrap_or_default(),
            visible,
            hidden_kind,
            topmost,
        })
        .collect()
}

unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `found` vector `top_level` passed in, alive for the call.
    let found = unsafe { &mut *(lparam.0 as *mut Vec<(u32, bool, bool, bool)>) };
    let mut pid = 0u32;
    // SAFETY: plain window queries on a handle EnumWindows gave us.
    let visible = unsafe {
        GetWindowThreadProcessId(hwnd, Some(&raw mut pid));
        IsWindowVisible(hwnd).as_bool()
    };
    // SAFETY: as above.
    let ex = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
    let tool = ex & WS_EX_TOOLWINDOW.0 != 0;
    let mut cloaked = 0u32;
    // SAFETY: `cloaked` is a u32, the size DWMWA_CLOAKED writes.
    let _ = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&raw mut cloaked).cast(),
            size_of::<u32>() as u32,
        )
    };
    found.push((pid, visible, tool || cloaked != 0, ex & WS_EX_TOPMOST.0 != 0));
    TRUE
}

/// A process handle that only asks (works across users and elevation, unlike reading its
/// modules), closed on drop.
struct Process(HANDLE);

impl Process {
    fn open(pid: u32) -> Option<Self> {
        // SAFETY: no pointers; the handle is closed in `drop`.
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
            .ok()
            .map(Self)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: the handle came from OpenProcess and is closed once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// The full path of the process's exe.
pub fn exe_path(pid: u32) -> Option<PathBuf> {
    let p = Process::open(pid)?;
    let mut buf = vec![0u16; 32 * 1024];
    let mut len = buf.len() as u32;
    // SAFETY: `buf` holds `len` u16s; Windows writes at most that many and sets `len`.
    unsafe { QueryFullProcessImageNameW(p.0, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &raw mut len) }
        .ok()?;
    let path = PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
    path.is_file().then_some(path)
}

/// The process's command line (from its PEB, through ProcessCommandLineInformation).
pub fn command_line(pid: u32) -> Option<String> {
    let p = Process::open(pid)?;
    let mut need = 0u32;
    // SAFETY: a size query: a null buffer of length 0, Windows only writes `need`.
    let _ = unsafe {
        NtQueryInformationProcess(
            p.0,
            ProcessCommandLineInformation,
            std::ptr::null_mut(),
            0,
            &raw mut need,
        )
    };
    if need < size_of::<UNICODE_STRING>() as u32 || need > 1 << 20 {
        return None;
    }
    // u64s, so the UNICODE_STRING at its start is aligned.
    let mut buf = vec![0u64; (need as usize).div_ceil(8)];
    let len = (buf.len() * 8) as u32;
    // SAFETY: `buf` is `len` bytes, 8-aligned; Windows writes a UNICODE_STRING whose buffer
    // points into `buf` after it.
    let status = unsafe {
        NtQueryInformationProcess(
            p.0,
            ProcessCommandLineInformation,
            buf.as_mut_ptr().cast(),
            len,
            &raw mut need,
        )
    };
    if status.is_err() {
        return None;
    }
    // SAFETY: on success the start of `buf` is an initialised UNICODE_STRING.
    let us = unsafe { &*(buf.as_ptr().cast::<UNICODE_STRING>()) };
    let start = buf.as_ptr() as usize;
    let at = us.Buffer.0 as usize;
    let chars = usize::from(us.Length) / 2;
    // Only trust a buffer that lies inside ours.
    if us.Buffer.is_null() || at < start || at + chars * 2 > start + buf.len() * 8 {
        return None;
    }
    // SAFETY: checked above: `chars` u16s at `at` are inside `buf`.
    let text = unsafe { std::slice::from_raw_parts(us.Buffer.0, chars) };
    Some(String::from_utf16_lossy(text))
}

const CLIENTS: &str = r"SOFTWARE\Clients\StartMenuInternet";

/// The browsers Windows lists (current user, then the machine) as (lowercase exe stem, exe).
pub fn installed() -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for key in subkeys(root, CLIENTS) {
            let Some(cmd) = string(root, &format!(r"{CLIENTS}\{key}\shell\open\command")) else {
                continue;
            };
            let exe = PathBuf::from(super::exe_of(&cmd));
            let Some(name) = exe.file_name().map(|n| super::stem(&n.to_string_lossy())) else {
                continue;
            };
            if !name.is_empty() && !out.iter().any(|(n, _)| *n == name) {
                out.push((name, exe));
            }
        }
    }
    out
}

fn subkeys(root: HKEY, path: &str) -> Vec<String> {
    let mut key = HKEY::default();
    // SAFETY: `key` receives the opened key, closed below.
    if unsafe { RegOpenKeyExW(root, &HSTRING::from(path), None, KEY_READ, &raw mut key) } != ERROR_SUCCESS {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..64 {
        let mut name = [0u16; 256];
        let mut len = name.len() as u32;
        // SAFETY: `name` holds `len` u16s; Windows writes at most that and sets `len`.
        let r = unsafe {
            RegEnumKeyExW(
                key,
                i,
                Some(PWSTR(name.as_mut_ptr())),
                &raw mut len,
                None,
                None,
                None,
                None,
            )
        };
        if r != ERROR_SUCCESS {
            break;
        }
        out.push(String::from_utf16_lossy(&name[..len as usize]));
    }
    // SAFETY: `key` was opened above and is closed once.
    let _ = unsafe { RegCloseKey(key) };
    out
}

/// A key's default string value.
fn string(root: HKEY, path: &str) -> Option<String> {
    let mut buf = [0u16; 2048];
    let mut bytes = (buf.len() * 2) as u32;
    // SAFETY: `buf` is `bytes` bytes; Windows writes at most that, NUL-terminated.
    let r = unsafe {
        RegGetValueW(
            root,
            &HSTRING::from(path),
            PCWSTR::null(),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&raw mut bytes),
        )
    };
    if r != ERROR_SUCCESS {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}
