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

#[cfg(test)]
mod tests {
    use super::*;

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
