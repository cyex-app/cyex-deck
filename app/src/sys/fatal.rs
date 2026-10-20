//! A plain Windows message box for errors before any window exists (the release build has
//! no console). Blocks until it's closed.

#[cfg(windows)]
pub fn show(title: &str, text: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW,
    };
    use windows::core::HSTRING;

    let (text, title) = (HSTRING::from(text), HSTRING::from(title));
    // SAFETY: two valid wide strings that outlive the call, no owner window.
    let _ = unsafe { MessageBoxW(None, &text, &title, MB_OK | MB_ICONERROR | MB_SETFOREGROUND) };
}

#[cfg(not(windows))]
pub fn show(_title: &str, _text: &str) {}
