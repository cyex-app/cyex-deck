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

/// P4 #16 Where a mock that is set but refused sends: nothing listens on port 0, so every
/// call fails at once. Never Twitch instead.
#[cfg(feature = "mocks")]
const NOWHERE: &str = "http://127.0.0.1:0";

/// P4 #16 `http` for one area: every Helix and GQL call of it to the mock named by the
/// first of `vars` that is set, with a fake sign-in (`gated`'s rules). Unset: `http` as is.
/// Mocks builds only.
#[cfg(feature = "mocks")]
pub fn http_via(app: &tauri::AppHandle, http: cyex_twitch::Http, vars: &[&str], what: &str) -> cyex_twitch::Http {
    via(http, gated(app, vars, what))
}

#[cfg(feature = "mocks")]
fn via(http: cyex_twitch::Http, pick: Result<Option<String>, String>) -> cyex_twitch::Http {
    match pick {
        Ok(None) => http,
        Ok(Some(base)) => http.mocked(&base),
        Err(_) => http.mocked(NOWHERE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `"CYEX_…_MOCK"` named in Rust source under `dir`.
    fn mock_vars(dir: &std::path::Path, out: &mut std::collections::BTreeSet<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                mock_vars(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let text = std::fs::read_to_string(&p).unwrap();
                for part in text.split('"').skip(1).step_by(2) {
                    let plain = part.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
                    if plain && part.starts_with("CYEX_") && part.ends_with("_MOCK") {
                        out.insert(part.to_owned());
                    }
                }
            }
        }
    }

    #[test]
    fn p4_16_release_refuses_every_mock_var() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut vars = std::collections::BTreeSet::new();
        mock_vars(&root.join("app/src"), &mut vars);
        mock_vars(&root.join("crates"), &mut vars);
        let release = std::fs::read_to_string(root.join("tools/release.ps1")).unwrap();
        assert!(vars.contains("CYEX_MODVIEW_MOCK"), "{vars:?}");
        for v in &vars {
            assert!(release.contains(&format!("'{v}'")), "{v} missing from tools/release.ps1's refused list");
        }
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn p4_16_an_area_goes_to_its_mock_or_nowhere() {
        let http = || cyex_twitch::Http::new().unwrap();
        assert_eq!(via(http(), Ok(None)).mock_base(), None, "unset: Twitch");
        let to = via(http(), pick(Some("http://127.0.0.1:5240/"), true, "mod view"));
        assert_eq!(to.mock_base(), Some("http://127.0.0.1:5240"));
        for refused in [
            pick(Some("http://127.0.0.1:5240"), false, "mod view"),
            pick(Some("https://api.twitch.tv"), true, "mod view"),
            pick(Some("http://127.0.0.1:1@evil.example"), true, "mod view"),
        ] {
            assert!(refused.is_err());
            assert_eq!(via(http(), refused).mock_base(), Some(NOWHERE), "refused: nowhere, never Twitch");
        }
    }

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
