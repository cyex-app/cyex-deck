//! V1061 tests: the mock HLS server (`tools/mock/hls.mjs`). Only in a test data folder,
//! only on this machine; real streams never see it.

use tauri::AppHandle;

/// `CYEX_HLS_MOCK`, when the app runs on a test data folder.
pub fn base(app: &AppHandle) -> Option<String> {
    crate::paths::webview(app)?;
    crate::mock_url::loopback_base(&std::env::var("CYEX_HLS_MOCK").ok()?)
}

/// The mock's DVR answer: `GET {base}/dvr/{login}` -> `{"vodID": "..." | null}`.
pub async fn dvr(http: &cyex_twitch::Http, base: &str, login: &str) -> Option<String> {
    let text = cyex_twitch::usher::text(http, &format!("{base}/dvr/{login}"))
        .await
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v["vodID"].as_str().map(str::to_owned)
}
