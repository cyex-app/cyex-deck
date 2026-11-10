//! V109-T01 The theme routes of api.cyex.app. No sign-in; changes need the edit key.
//! Errors come as `{ error, message }` and the message is shown as it is.

use std::collections::BTreeMap;

use cyex_settings::schema::{CustomTheme, Fetched, Upstream, date_of};
use cyex_twitch::Http;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use super::form::{self, Picture};

pub const BASE: &str = "https://api.cyex.app";
/// A theme answer is a few KB; a list of 50 updates less.
const MAX_ANSWER: usize = 256 * 1024;
const OFFLINE: &str = "Couldn’t reach cyex.app. Try again in a bit.";
/// The most ids one update check asks about.
pub const MAX_UPDATE_IDS: usize = 50;

pub struct Api {
    pub http: Http,
    pub base: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    pub id: String,
    pub edit_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct View {
    version: i64,
    theme: CustomTheme,
    #[serde(default)]
    updated_at: String,
    /// V110-G1 A moderator removed the wallpaper (absent otherwise).
    #[serde(default)]
    wallpaper_removed: bool,
}

#[derive(Deserialize)]
struct UpdateState {
    #[serde(default)]
    version: Option<i64>,
    #[serde(default)]
    removed: bool,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default)]
    message: String,
}

/// The server's own message, else a short one for the status.
pub fn error_text(status: u16, body: &[u8]) -> String {
    if let Ok(e) = serde_json::from_slice::<ErrorBody>(body)
        && !e.message.trim().is_empty()
    {
        return e.message.trim().chars().take(300).collect();
    }
    match status {
        404 => "That theme isn’t there. Check the link.".into(),
        410 => "The author removed this theme.".into(),
        401 => "This PC doesn’t have the key to change that theme.".into(),
        413 => "That’s too big to share.".into(),
        429 => "Slow down a little and try again.".into(),
        _ => format!("cyex.app said no ({status}). Try again later."),
    }
}

fn version(v: i64) -> u32 {
    u32::try_from(v.max(0)).unwrap_or(u32::MAX)
}

fn fetched(view: View) -> Fetched {
    let mut theme = view.theme.shareable();
    theme.sanitize();
    Fetched {
        version: version(view.version),
        date: date_of(&view.updated_at),
        theme,
        wallpaper_removed: view.wallpaper_removed,
    }
}

/// `{ a: { version }, b: { removed: true }, c: null }` → what each known id says.
pub fn read_updates(body: &[u8]) -> Option<BTreeMap<String, Upstream>> {
    let map: BTreeMap<String, Option<UpdateState>> = serde_json::from_slice(body).ok()?;
    Some(
        map.into_iter()
            .filter_map(|(id, s)| {
                let s = s?;
                let up = if s.removed {
                    Upstream::Removed
                } else {
                    Upstream::Version(version(s.version?))
                };
                Some((id, up))
            })
            .collect(),
    )
}

/// A call that didn't work: the status (0 when offline) and what to show.
#[derive(Debug)]
pub struct Fail {
    pub status: u16,
    pub message: String,
}

impl From<Fail> for String {
    fn from(f: Fail) -> Self {
        f.message
    }
}

fn odd() -> Fail {
    Fail {
        status: 0,
        message: "cyex.app sent something odd. Try again later.".into(),
    }
}

impl Api {
    fn url(&self, path: &str) -> String {
        format!("{}/themes/v1{path}", self.base)
    }

    async fn call(
        &self,
        method: &str,
        path: &str,
        key: Option<&str>,
        body: Option<(String, Vec<u8>)>,
    ) -> Result<Vec<u8>, Fail> {
        let (status, bytes) = self
            .http
            .send(method, &self.url(path), key, body, MAX_ANSWER)
            .await
            .map_err(|_| Fail {
                status: 0,
                message: OFFLINE.into(),
            })?;
        if (200..300).contains(&status) {
            Ok(bytes)
        } else {
            Err(Fail {
                status,
                message: error_text(status, &bytes),
            })
        }
    }

    async fn json<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        key: Option<&str>,
        body: Option<(String, Vec<u8>)>,
    ) -> Result<T, Fail> {
        let bytes = self.call(method, path, key, body).await?;
        serde_json::from_slice(&bytes).map_err(|_| odd())
    }

    fn form(theme: &CustomTheme, picture: &Picture) -> (String, Vec<u8>) {
        let json = serde_json::to_string(&theme.shareable()).expect("theme serializes");
        let boundary = format!("cyex-{}", cyex_settings::schema::random_id());
        form::build(&json, picture, &boundary)
    }

    pub async fn create(&self, theme: &CustomTheme, picture: &Picture) -> Result<Created, Fail> {
        self.json("POST", "", None, Some(Self::form(theme, picture))).await
    }

    /// The version the PUT made.
    pub async fn update(&self, id: &str, key: &str, theme: &CustomTheme, picture: &Picture) -> Result<u32, Fail> {
        let view: View = self
            .json("PUT", &format!("/{id}"), Some(key), Some(Self::form(theme, picture)))
            .await?;
        Ok(version(view.version))
    }

    pub async fn delete(&self, id: &str, key: &str) -> Result<(), Fail> {
        self.call("DELETE", &format!("/{id}"), Some(key), None).await.map(|_| ())
    }

    pub async fn get(&self, id: &str) -> Result<Fetched, Fail> {
        self.json("GET", &format!("/{id}"), None, None).await.map(fetched)
    }

    pub async fn updates(&self, ids: &[String]) -> Result<BTreeMap<String, Upstream>, Fail> {
        let bytes = self
            .call("GET", &format!("/updates?ids={}", ids.join(",")), None, None)
            .await?;
        read_updates(&bytes).ok_or_else(odd)
    }

    pub async fn report(&self, id: &str, reason: &str) -> Result<(), Fail> {
        let body = serde_json::to_vec(&json!({ "reason": reason })).expect("json");
        self.call("POST", &format!("/{id}/report"), None, Some(("application/json".into(), body)))
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_messages_win() {
        let body = br#"{"error":"bad_theme","message":"colors.bg: not a color"}"#;
        assert_eq!(error_text(400, body), "colors.bg: not a color");
        assert_eq!(error_text(429, b""), "Slow down a little and try again.");
        assert_eq!(error_text(500, b"<html>"), "cyex.app said no (500). Try again later.");
    }

    #[test]
    fn update_answers() {
        let body = br#"{"k3f9x2ab":{"version":4,"updatedAt":"2026-11-05T10:00:00Z"},"m4n5p6q7":{"removed":true},"zzzzzzzz":null}"#;
        let m = read_updates(body).unwrap();
        assert_eq!(m.get("k3f9x2ab"), Some(&Upstream::Version(4)));
        assert_eq!(m.get("m4n5p6q7"), Some(&Upstream::Removed));
        assert!(!m.contains_key("zzzzzzzz"), "unknown: no news, not removed");
        assert!(read_updates(b"[]").is_none());
    }

    #[test]
    fn a_fetched_theme_is_clean_and_dated() {
        let body = br##"{"id":"k3f9x2ab","version":3,"theme":{"name":"Neon","base":"dark","colors":{"bg":"#ABC"},"accent":"#ff00ff","wallpaper":{"source":"https://api.cyex.app/themes/v1/img/a.webp","area":"window","dim":70,"blur":0,"fit":"cover"}},"wallpaper":"https://api.cyex.app/themes/v1/img/a.webp","createdAt":"2026-11-01T00:00:00Z","updatedAt":"2026-11-04T08:00:00Z"}"##;
        let f = fetched(serde_json::from_slice(body).unwrap());
        assert_eq!(f.version, 3);
        assert_eq!(f.date, "2026/11/04");
        assert_eq!(f.theme.colors["bg"], "#aabbcc");
        assert!(f.theme.wallpaper.is_link(), "the server's picture is a link wallpaper");
        assert!(f.theme.id.is_empty() && f.theme.link.is_empty());
    }

    #[test]
    fn a_removed_wallpaper_is_read() {
        let body = br##"{"id":"k3f9x2ab","version":4,"theme":{"name":"Neon","base":"dark","colors":{},"accent":"#ff00ff","wallpaper":{"source":"","area":"window","dim":70,"blur":0,"fit":"cover"}},"wallpaper":null,"wallpaperRemoved":true,"createdAt":"2026-11-01T00:00:00Z","updatedAt":"2026-11-10T08:00:00Z"}"##;
        let f = fetched(serde_json::from_slice(body).unwrap());
        assert!(f.wallpaper_removed);
        assert!(f.theme.wallpaper.source.is_empty());
        let plain = br##"{"id":"k3f9x2ab","version":4,"theme":{"name":"Neon","base":"dark","colors":{},"accent":"#ff00ff"},"wallpaper":null}"##;
        assert!(!fetched(serde_json::from_slice(plain).unwrap()).wallpaper_removed);
    }

    #[test]
    fn the_form_never_carries_the_link() {
        let mut t = CustomTheme {
            id: "local1".into(),
            ..CustomTheme::default()
        };
        t.link.shared_id = "k3f9x2ab".into();
        let (_, body) = Api::form(&t, &Picture::Keep);
        let text = String::from_utf8(body).unwrap();
        assert!(!text.contains("k3f9x2ab") && !text.contains("local1") && !text.contains("link"));
    }
}
