//! V1061 tests: the mock HLS server (`tools/mock/hls.mjs`). Only in a test data folder,
//! only on this machine; real streams never see it.

use tauri::AppHandle;

/// `CYEX_HLS_MOCK`, when the app runs on a test data folder.
pub fn base(app: &AppHandle) -> Option<String> {
    crate::paths::webview(app)?;
    allowed(&std::env::var("CYEX_HLS_MOCK").ok()?)
}

fn allowed(url: &str) -> Option<String> {
    (url.starts_with("http://127.0.0.1:") || url.starts_with("http://localhost:"))
        .then(|| url.trim_end_matches('/').to_owned())
}

/// The mock's DVR answer: `GET {base}/dvr/{login}` -> `{"vodID": "..." | null}`.
pub async fn dvr(http: &cyex_twitch::Http, base: &str, login: &str) -> Option<String> {
    let text = cyex_twitch::usher::text(http, &format!("{base}/dvr/{login}"))
        .await
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v["vodID"].as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_base_only_local() {
        assert_eq!(
            allowed("http://127.0.0.1:8123").as_deref(),
            Some("http://127.0.0.1:8123")
        );
        assert_eq!(
            allowed("http://localhost:8123/").as_deref(),
            Some("http://localhost:8123")
        );
        assert_eq!(allowed("https://127.0.0.1:8123"), None);
        assert_eq!(allowed("https://example.com"), None);
        assert_eq!(allowed("http://10.0.0.1:8123"), None);
    }
}
