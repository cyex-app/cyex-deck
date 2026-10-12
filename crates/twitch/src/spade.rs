//! Watch time (1.30, W1): the `minute-watched` events twitch.tv's player sends to spade.
//! They are how Twitch counts watching for channel points, watch streaks and drops. Like
//! twitch-channel-points-miner-v2, each one follows a real look at the stream: the
//! lowest quality's playlist and a HEAD of its newest segment.

use base64::Engine;
use serde_json::json;

use crate::gql::AccessToken;
use crate::{Error, Http, Result, usher};

/// What twitch.tv's settings script named in 2026/10; used when it can't be read.
pub const URL: &str = "https://spade.twitch.tv/track";
const PAGE: &str = "https://www.twitch.tv/";
const SETTINGS: &str = "https://assets.twitch.tv/config/settings.";
const PAGE_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:85.0) Gecko/20100101 Firefox/85.0";

/// One minute of watching.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Minute<'a> {
    pub channel_id: &'a str,
    pub login: &'a str,
    pub broadcast_id: &'a str,
    pub user_id: &'a str,
    /// Drops count only with the game (id, name).
    pub game: Option<(&'a str, &'a str)>,
}

/// spade's address, read from twitch.tv's settings script.
pub async fn find_url(http: &Http) -> String {
    let headers = [("User-Agent".to_owned(), PAGE_AGENT.to_owned())];
    let Ok(page) = http.get_text(PAGE, &headers, 2_000_000).await else {
        return URL.into();
    };
    let Some(script) = settings_script(&page) else {
        return URL.into();
    };
    match http.get_text(script, &headers, 2_000_000).await {
        Ok(js) => spade_url(&js).unwrap_or(URL).to_owned(),
        Err(_) => URL.into(),
    }
}

fn settings_script(page: &str) -> Option<&str> {
    let start = page.find(SETTINGS)?;
    let end = start + page[start..].find(".js")? + 3;
    Some(&page[start..end])
}

fn spade_url(js: &str) -> Option<&str> {
    const KEY: &str = "\"spade_url\":\"";
    let start = js.find(KEY)? + KEY.len();
    let len = js[start..].find('"')?;
    Some(&js[start..start + len]).filter(|u| u.starts_with("https://"))
}

/// The form value spade takes: base64 of the event list.
pub fn payload(m: &Minute<'_>) -> String {
    let mut props = json!({
        "channel_id": m.channel_id,
        "broadcast_id": m.broadcast_id,
        "player": "site",
        "user_id": m.user_id,
        "live": true,
        "channel": m.login,
    });
    if let Some((id, name)) = m.game {
        props["game"] = json!(name);
        props["game_id"] = json!(id);
    }
    let events = json!([{ "event": "minute-watched", "properties": props }]);
    base64::engine::general_purpose::STANDARD.encode(events.to_string())
}

/// Looks at the stream the way a player would: the master playlist, the lowest
/// quality's playlist, then the head of its newest segment.
pub async fn touch(http: &Http, login: &str, token: &AccessToken, user_agent: &str) -> Result<()> {
    let master = usher::text(http, &usher::live_url(login, token, "h264")).await?;
    let media_url = last_uri(&master).ok_or_else(|| broken("no variant"))?;
    let media = usher::text(http, media_url).await?;
    let segment = last_uri(&media).ok_or_else(|| broken("no segment"))?;
    match http.head(segment, user_agent).await? {
        200..=299 => Ok(()),
        status => Err(Error::Api {
            status,
            message: "segment refused".into(),
        }),
    }
}

/// Sends one minute: spade answers 204 when it took it.
pub async fn send(http: &Http, url: &str, payload: &str, user_agent: &str) -> Result<u16> {
    http.post_form(url, &[("data", payload)], user_agent).await
}

/// The last line of a playlist that isn't a tag: the lowest quality in a master, the
/// newest segment in a media playlist.
fn last_uri(playlist: &str) -> Option<&str> {
    playlist
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty() && !l.starts_with('#'))
        .filter(|l| l.starts_with("https://"))
}

fn broken(what: &str) -> Error {
    Error::Api {
        status: 200,
        message: what.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_settings_script_and_spade() {
        let page =
            r#"<link rel="preload" href="https://assets.twitch.tv/config/settings.a346c0e2.js" as="script">"#;
        assert_eq!(
            settings_script(page),
            Some("https://assets.twitch.tv/config/settings.a346c0e2.js")
        );
        assert_eq!(
            spade_url(r#"{"a":1,"spade_url":"https://spade.twitch.tv/track","b":2}"#),
            Some(URL)
        );
        assert_eq!(spade_url(r#"{"spade_url":"javascript:x"}"#), None);
        assert_eq!(settings_script("<html></html>"), None);
    }

    #[test]
    fn payload_is_base64_json_with_the_game() {
        let m = Minute {
            channel_id: "1",
            login: "a",
            broadcast_id: "9",
            user_id: "2",
            game: Some(("7", "Chess")),
        };
        let raw = base64::engine::general_purpose::STANDARD
            .decode(payload(&m))
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(v[0]["event"], "minute-watched");
        let p = &v[0]["properties"];
        assert_eq!(
            (
                p["broadcast_id"].as_str(),
                p["game_id"].as_str(),
                p["live"].as_bool()
            ),
            (Some("9"), Some("7"), Some(true))
        );
        let none = Minute { game: None, ..m };
        let raw = base64::engine::general_purpose::STANDARD
            .decode(payload(&none))
            .unwrap();
        assert!(!String::from_utf8(raw).unwrap().contains("game"));
    }

    #[test]
    fn playlists_give_their_last_uri() {
        let master = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nhttps://a/720.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2\nhttps://a/160.m3u8\n";
        assert_eq!(last_uri(master), Some("https://a/160.m3u8"));
        let media = "#EXTM3U\n#EXTINF:2.0,live\nhttps://v/1.ts\n#EXTINF:2.0,live\nhttps://v/2.ts\n#EXT-X-TWITCH-PREFETCH:https://v/3.ts";
        assert_eq!(last_uri(media), Some("https://v/2.ts"));
        assert_eq!(last_uri("#EXTM3U\n"), None);
    }
}
