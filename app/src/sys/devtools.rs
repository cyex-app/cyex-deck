//! V1051-02 One DevTools Protocol call in a window's page (WebView2's
//! `CallDevToolsProtocolMethod`), e.g. clearing 7TV's origins from a shared profile.

/// The call's JSON answer, or `None` (failed, timed out, or not on Windows).
#[cfg(windows)]
pub async fn call(window: &tauri::WebviewWindow, method: &'static str, params: String) -> Option<String> {
    use std::sync::Mutex;
    use webview2_com::CallDevToolsProtocolMethodCompletedHandler;
    use windows::core::HSTRING;

    let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
    let tx = Mutex::new(Some(tx));
    window
        .with_webview(move |w| {
            // SAFETY: plain COM calls on the controller Tauri hands us, on the UI thread.
            let core = unsafe { w.controller().CoreWebView2() };
            let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |res, json| {
                let tx = tx.lock().unwrap_or_else(|p| p.into_inner()).take();
                if let Some(tx) = tx {
                    let _ = tx.send(res.is_ok().then_some(json));
                }
                Ok(())
            }));
            if let Ok(core) = core {
                // SAFETY: the strings and handler outlive the call.
                let _ = unsafe { core.CallDevToolsProtocolMethod(&HSTRING::from(method), &HSTRING::from(params), &handler) };
            }
        })
        .ok()?;
    tokio::time::timeout(std::time::Duration::from_secs(5), rx).await.ok()?.ok()?
}

#[cfg(not(windows))]
pub async fn call(_window: &tauri::WebviewWindow, _method: &'static str, _params: String) -> Option<String> {
    None
}
