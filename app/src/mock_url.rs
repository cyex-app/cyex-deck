//! R11 The one rule for local mock URLs: http://127.0.0.1:<port> only (mocks builds).

/// A bare `http://127.0.0.1:<port>`: no user part, path, query or fragment.
pub fn loopback_base(raw: &str) -> Option<String> {
    let u = tauri::Url::parse(raw).ok()?;
    let bare = matches!(u.path(), "" | "/") && u.query().is_none() && u.fragment().is_none();
    bare.then(|| loopback_origin(&u)).flatten()
}

/// `http://127.0.0.1:<port>` and nothing else: no other host, no user part
/// (`http://127.0.0.1:1@elsewhere` is elsewhere).
pub fn loopback_origin(u: &tauri::Url) -> Option<String> {
    let plain = u.scheme() == "http"
        && u.host_str() == Some("127.0.0.1")
        && u.username().is_empty()
        && u.password().is_none();
    plain.then(|| u.port().map(|p| format!("http://127.0.0.1:{p}")))?
}

/// V107-H6a A GQL mock named by the first of `vars` that is set, for `what` ("cheer"):
/// none when unset; set, it needs a test data folder and a loopback address, or nothing is
/// sent at all (never Twitch with a real token instead). Mocks builds only.
#[cfg(feature = "mocks")]
pub fn gated(app: &tauri::AppHandle, vars: &[&str], what: &str) -> Result<Option<String>, String> {
    let raw = vars.iter().find_map(|v| std::env::var(v).ok());
    pick(raw.as_deref(), crate::paths::webview(app).is_some(), what)
}

#[cfg(feature = "mocks")]
pub(crate) fn pick(raw: Option<&str>, test_data: bool, what: &str) -> Result<Option<String>, String> {
    let Some(raw) = raw.filter(|r| !r.trim().is_empty()) else {
        return Ok(None);
    };
    if !test_data {
        return Err(format!("The {what} mock runs only on a test data folder."));
    }
    loopback_base(raw)
        .map(Some)
        .ok_or_else(|| format!("The {what} mock must be http://127.0.0.1:<port>."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "mocks")]
    #[test]
    fn v107_r41_a_mock_needs_test_data_and_loopback() {
        assert_eq!(pick(None, false, "GQL"), Ok(None));
        assert_eq!(pick(Some(" "), true, "GQL"), Ok(None));
        assert_eq!(pick(Some("http://127.0.0.1:5231"), true, "GQL"), Ok(Some("http://127.0.0.1:5231".into())));
        assert_eq!(
            pick(Some("http://127.0.0.1:5231"), false, "cheer").unwrap_err(),
            "The cheer mock runs only on a test data folder.",
            "real data: refused, not sent to Twitch"
        );
        assert!(pick(Some("http://evil.example:5231"), true, "GQL").is_err());
        assert!(pick(Some("http://127.0.0.1:1@evil.example"), true, "GQL").is_err());
    }

    #[test]
    fn only_ever_this_pc() {
        let ok = |raw: &str, want: &str| assert_eq!(loopback_base(raw).as_deref(), Some(want));
        ok("http://127.0.0.1:8123", "http://127.0.0.1:8123");
        ok("http://127.0.0.1:5199/", "http://127.0.0.1:5199");
        for raw in [
            "http://localhost:8123/",
            "http://localhost:5199",
            "https://127.0.0.1:8123",
            "https://example.com",
            "http://10.0.0.1:8123",
            "http://127.0.0.1:1@evil.example/",
            "http://127.0.0.1:5199@evil.example",
            "http://127.0.0.1:1:2@evil.example/",
            "http://user@127.0.0.1:8123",
            "http://127.0.0.1:8123/x",
            "http://127.0.0.1:8123/?a=1",
            "http://127.0.0.1:8123/#f",
            "http://127.0.0.1:5199/#f",
            "http://127.0.0.1",
            "http://127.0.0.1.evil.example:8123",
        ] {
            assert_eq!(loopback_base(raw), None, "{raw}");
        }
    }
}
