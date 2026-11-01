//! V108-N26 What Discord shows: the channel you watch, as a SET_ACTIVITY frame body.

use serde::Deserialize;
use serde_json::{Value, json};
use ts_rs::TS;

/// Key of the app's logo in the Discord app's art assets.
const LOGO_KEY: &str = "logo";
const APP_NAME: &str = "cyex deck";
const BUTTON: &str = "Watch on Twitch";
const MAX_TEXT: usize = 100;
const MAX_URL: usize = 256;

/// The channel being watched, as the window reports it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Watch {
    pub login: String,
    /// Display name (CJK and capitals kept).
    pub name: String,
    /// The channel's profile picture, https.
    pub avatar: Option<String>,
}

/// Discord's activity types we use: Playing is the fallback when Watching is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Playing = 0,
    Watching = 3,
}

fn cut(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

fn valid_login(login: &str) -> bool {
    !login.is_empty() && login.len() <= 25 && login.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Whether Discord can get this picture: https, a sane length.
fn valid_avatar(url: &str) -> bool {
    url.starts_with("https://") && url.len() <= MAX_URL && !url.contains(char::is_whitespace)
}

/// The `activity` object. `since` is seconds since the epoch.
pub fn activity(w: &Watch, since: u64, kind: Kind) -> Value {
    let name = cut(&w.name, MAX_TEXT);
    let name = if name.is_empty() { w.login.clone() } else { name };
    let mut assets = json!({ "large_image": LOGO_KEY, "large_text": APP_NAME });
    if let Some(url) = w.avatar.as_deref().filter(|u| valid_avatar(u)) {
        assets["small_image"] = json!(url);
        // Discord drops texts under two characters.
        if name.chars().count() >= 2 {
            assets["small_text"] = json!(name);
        }
    }
    let mut a = json!({
        "type": kind as u8,
        "details": format!("Watching {name}"),
        "assets": assets,
        "timestamps": { "start": since },
    });
    if valid_login(&w.login) {
        a["buttons"] = json!([{ "label": BUTTON, "url": format!("https://www.twitch.tv/{}", w.login) }]);
    }
    a
}

/// The whole command frame body.
pub fn set_activity(pid: u32, nonce: &str, activity: Option<Value>) -> Vec<u8> {
    let body = json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": pid, "activity": activity },
        "nonce": nonce,
    });
    body.to_string().into_bytes()
}

pub fn handshake(client_id: &str) -> Vec<u8> {
    json!({ "v": 1, "client_id": client_id }).to_string().into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch(avatar: Option<&str>) -> Watch {
        Watch { login: "somebody".into(), name: "SomeBody".into(), avatar: avatar.map(str::to_owned) }
    }

    #[test]
    fn full_activity() {
        let a = activity(&watch(Some("https://static-cdn.jtvnw.net/p.png")), 1_700_000_000, Kind::Watching);
        assert_eq!(a["type"], 3);
        assert_eq!(a["details"], "Watching SomeBody");
        assert_eq!(a["assets"]["large_image"], "logo");
        assert_eq!(a["assets"]["large_text"], "cyex deck");
        assert_eq!(a["assets"]["small_image"], "https://static-cdn.jtvnw.net/p.png");
        assert_eq!(a["assets"]["small_text"], "SomeBody");
        assert_eq!(a["timestamps"]["start"], 1_700_000_000u64);
        assert_eq!(a["buttons"][0]["label"], "Watch on Twitch");
        assert_eq!(a["buttons"][0]["url"], "https://www.twitch.tv/somebody");
        assert!(a.get("state").is_none());
    }

    #[test]
    fn without_avatar_or_with_a_bad_one() {
        for av in [None, Some("http://x/p.png"), Some("")] {
            let a = activity(&watch(av), 1, Kind::Playing);
            assert_eq!(a["type"], 0);
            assert!(a["assets"].get("small_image").is_none());
            assert!(a["assets"].get("small_text").is_none());
            assert_eq!(a["assets"]["large_image"], "logo");
        }
    }

    #[test]
    fn button_needs_a_clean_login() {
        let mut w = watch(None);
        w.login = "bad/../login".into();
        assert!(activity(&w, 1, Kind::Watching).get("buttons").is_none());
        w.login = String::new();
        assert!(activity(&w, 1, Kind::Watching).get("buttons").is_none());
    }

    #[test]
    fn names_are_cut_and_never_empty() {
        let mut w = watch(Some("https://x/p.png"));
        w.name = "\u{3042}".repeat(300);
        let a = activity(&w, 1, Kind::Watching);
        assert_eq!(a["assets"]["small_text"].as_str().unwrap().chars().count(), MAX_TEXT);
        w.name = "  ".into();
        assert_eq!(activity(&w, 1, Kind::Watching)["details"], "Watching somebody");
        w.name = "x".into();
        assert!(activity(&w, 1, Kind::Watching)["assets"].get("small_text").is_none());
    }

    #[test]
    fn frames() {
        let v: Value = serde_json::from_slice(&set_activity(7, "n1", None)).unwrap();
        assert_eq!(v["cmd"], "SET_ACTIVITY");
        assert_eq!(v["args"]["pid"], 7);
        assert!(v["args"]["activity"].is_null());
        assert_eq!(v["nonce"], "n1");
        let h: Value = serde_json::from_slice(&handshake("123")).unwrap();
        assert_eq!(h, json!({ "v": 1, "client_id": "123" }));
    }
}
