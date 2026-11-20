//! cyex.tv over HTTP (V112-01..04): the global set, a channel's active set by Twitch id, and
//! the personal sets of up to 100 chatters at once. Only the fields the deck draws are read
//! (ADR 0013); the rest of an answer is ignored.

use cyex_chat::model::Provider;
use cyex_twitch::pace::{Lane, Opts, Service};
use cyex_twitch::{Error, Http, Result};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::model::{Emote, EmoteSet, Scope};

const API: &str = "https://api.cyex.tv/v1";
/// Where emote pictures are drawn from (the UI builds `https://cdn.cyex.tv/emotes/<key>/<name>`).
pub const CDN: &str = "cdn.cyex.tv";
/// Most users one personal-sets lookup takes (a bigger one is a 400).
pub const MAX_LOOKUP: usize = 100;

#[cfg(feature = "mocks")]
static BASE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
/// Test runs with a local mock: every cyex.tv read goes there.
#[cfg(feature = "mocks")]
pub fn use_mock(base: &str) {
    let _ = BASE.set(base.trim_end_matches('/').to_owned());
}
fn base() -> &'static str {
    #[cfg(feature = "mocks")]
    if let Some(b) = BASE.get() {
        return b;
    }
    API
}

/// V112-01 The global set.
pub async fn global(http: &Http) -> Result<EmoteSet> {
    parse_global(&global_raw(http).await?).map_err(Error::Decode)
}

/// The global set as cyex.tv sent it, for the app's disk copy; [`parse_global`] reads it.
pub async fn global_raw(http: &Http) -> Result<Value> {
    http.get_json_paced(Service::Cyextv, Lane::Front, &format!("{}/sets/global", base()))
        .await
}

pub fn parse_global(v: &Value) -> serde_json::Result<EmoteSet> {
    Ok(RawSet::deserialize(v)?.into_set("cyextv-global".to_owned(), Scope::Global))
}

/// V112-02 The channel's active set. `None` without a cyex.tv channel (404) or without an
/// active set. Paced; in the background it fails fast and the emote retry queue asks again.
pub async fn channel(http: &Http, twitch_id: &str, lane: Lane) -> Result<Option<EmoteSet>> {
    parse_channel(twitch_id, &channel_raw(http, twitch_id, lane).await?).map_err(Error::Decode)
}

/// A channel's answer and its active set's, as `{"channel": …, "set": …}` (what the disk copy
/// keeps); `null` when cyex.tv has no such channel (404 is an answer, not an error).
pub async fn channel_raw(http: &Http, twitch_id: &str, lane: Lane) -> Result<Value> {
    let opts = Opts {
        lane,
        quick: true,
        ..Opts::default()
    };
    let url = format!("{}/channels/twitch/{}", base(), enc(twitch_id));
    let channel: Value = match http.get_json_paced_with(Service::Cyextv, opts, &url).await {
        Err(Error::Api { status: 404, .. }) => return Ok(Value::Null),
        r => r?,
    };
    let Some(set_id) = channel
        .get("active_set_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return Ok(json!({ "channel": channel, "set": null }));
    };
    let url = format!("{}/sets/{}", base(), enc(set_id));
    let set: Value = match http.get_json_paced_with(Service::Cyextv, opts, &url).await {
        // The set went between the two reads: the channel has no emotes now.
        Err(Error::Api { status: 404, .. }) => Value::Null,
        r => r?,
    };
    Ok(json!({ "channel": channel, "set": set }))
}

/// A [`channel_raw`] answer; `None` without a channel or an active set.
pub fn parse_channel(twitch_id: &str, v: &Value) -> serde_json::Result<Option<EmoteSet>> {
    if v.is_null() {
        return Ok(None);
    }
    let answer = RawChannelAnswer::deserialize(v)?;
    Ok(answer
        .set
        .map(|set| set.into_set(format!("cyextv-{twitch_id}"), Scope::Channel)))
}

/// V112-04 The personal sets of these Twitch user ids (at most [`MAX_LOOKUP`]).
pub async fn personal_raw(http: &Http, user_ids: &[String], lane: Lane) -> Result<Value> {
    let ids = &user_ids[..user_ids.len().min(MAX_LOOKUP)];
    let body = json!({ "platform": "twitch", "user_ids": ids });
    let opts = Opts {
        lane,
        quick: true,
        ..Opts::default()
    };
    http.post_json_paced(
        Service::Cyextv,
        opts,
        &format!("{}/personal-sets/lookup", base()),
        &body,
    )
    .await
}

/// Each listed user's personal set (only users with one are listed). A set whose emotes all
/// lack pictures is left out.
pub fn parse_personal(v: &Value) -> serde_json::Result<Vec<(String, EmoteSet)>> {
    let list = RawLookup::deserialize(v)?;
    Ok(list
        .sets
        .into_iter()
        .map(|e| {
            let id = format!("cyextv-{}", e.set.id);
            (e.platform_user_id, e.set.into_set(id, Scope::Personal))
        })
        .filter(|(_, s)| !s.emotes.is_empty())
        .collect())
}

/// The key of an emote's picture folder, `https://cdn.cyex.tv/emotes/<key>`; `None` for
/// anything else: not https, another host, a user or port, another path.
pub fn picture_key(host: &str) -> Option<&str> {
    let rest = host.strip_prefix("https://")?;
    let (name, path) = rest.split_once('/')?;
    if name != CDN {
        return None;
    }
    let key = path.strip_prefix("emotes/")?;
    let ok = !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    ok.then_some(key)
}

/// `{host}/{name}` when `host` is an https folder on cdn.cyex.tv or api.cyex.tv and `name` is a
/// plain file name.
pub fn image_url(host: &str, name: &str) -> Option<String> {
    let rest = host.strip_prefix("https://")?;
    let (site, path) = rest.split_once('/').unwrap_or((rest, ""));
    if !matches!(site, "cdn.cyex.tv" | "api.cyex.tv") {
        return None;
    }
    let plain = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/'))
            && !s.split('/').any(|p| p.is_empty() || p == "." || p == "..")
    };
    if (!path.is_empty() && !plain(path)) || !plain(name) || name.contains('/') {
        return None;
    }
    Some(if path.is_empty() {
        format!("https://{site}/{name}")
    } else {
        format!("https://{site}/{path}/{name}")
    })
}

/// The deck's emote id: cyex.tv's id, a dot, and the picture folder's key, because the picture's
/// address can't be built from the emote id alone.
pub fn deck_id(emote_id: &str, key: &str) -> String {
    format!("{emote_id}.{key}")
}

/// V112-08 The cyex.tv emote id inside a [`deck_id`].
pub fn emote_id(deck_id: &str) -> &str {
    deck_id.split_once('.').map_or(deck_id, |(id, _)| id)
}

/// A path part, percent-escaped.
fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[derive(Deserialize)]
struct RawChannelAnswer {
    set: Option<RawSet>,
}

#[derive(Deserialize)]
struct RawLookup {
    #[serde(default)]
    sets: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    platform_user_id: String,
    set: RawSet,
}

#[derive(Deserialize)]
struct RawSet {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    emotes: Vec<RawItem>,
}

/// An emote as it sits in a set: its name there (the alias) and the set's zero-width choice.
#[derive(Deserialize)]
struct RawItem {
    name: String,
    #[serde(default)]
    zero_width: bool,
    added_at: Option<String>,
    added_by: Option<String>,
    emote: RawEmote,
}

#[derive(Deserialize)]
struct RawEmote {
    id: String,
    name: String,
    owner: Option<RawOwner>,
    #[serde(default)]
    flags: RawFlags,
    #[serde(default)]
    animated: bool,
    #[serde(default)]
    host: String,
    #[serde(default)]
    files: Vec<RawFile>,
}

#[derive(Deserialize)]
struct RawOwner {
    id: String,
    display_name: String,
}

#[derive(Default, Deserialize)]
struct RawFlags {
    #[serde(default)]
    private: bool,
    #[serde(default)]
    nsfw: bool,
}

#[derive(Deserialize)]
struct RawFile {
    name: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
}

impl RawSet {
    fn into_set(self, id: String, scope: Scope) -> EmoteSet {
        let name = (!self.name.is_empty()).then_some(self.name);
        let emotes = self.emotes.into_iter().filter_map(RawItem::emote).collect();
        let mut set = EmoteSet::new(id, Provider::Cyextv, scope, emotes);
        set.name = name;
        set
    }
}

impl RawItem {
    /// `None` for an emote without pictures yet, or whose address isn't cyex.tv's.
    fn emote(self) -> Option<Emote> {
        let e = self.emote;
        if e.files.is_empty() {
            return None;
        }
        let key = picture_key(&e.host)?;
        let one = e
            .files
            .iter()
            .find(|f| f.name == "1x.webp" && f.width > 0 && f.height > 0);
        let mut out = Emote::new(Provider::Cyextv, deck_id(&e.id, key), self.name);
        out.original = Some(e.name).filter(|n| *n != out.name);
        out.zero_width = self.zero_width;
        out.animated = e.animated;
        out.size = one.map(|f| {
            (
                f.width.min(u16::MAX.into()) as u16,
                f.height.min(u16::MAX.into()) as u16,
            )
        });
        out.owner = e.owner.as_ref().map(|o| o.display_name.clone());
        out.owner_id = e.owner.map(|o| o.id);
        out.added_at = self.added_at;
        out.added_by = self.added_by;
        out.private = e.flags.private;
        out.nsfw = e.flags.nsfw;
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/cyextv/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    const KEY: &str = "3f2a9c1d5e7b40a8b6c4d2e1f0a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4e3f2a1";

    #[test]
    fn global_keeps_the_alias_and_zero_width() {
        let set = parse_global(&fixture("global.json")).unwrap();
        assert_eq!(set.id, "cyextv-global");
        assert_eq!(set.scope, Scope::Global);
        let names: Vec<_> = set.emotes.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Hello", "wave", "Sparkle"]);
        let wave = set.get("wave").unwrap();
        assert_eq!(wave.original.as_deref(), Some("Wave"), "the set renamed it");
        assert!(set.get("Hello").unwrap().original.is_none());
        // The zero-width choice is the set's, not the emote's own flag.
        assert!(set.get("Sparkle").unwrap().zero_width);
        assert!(!set.get("Hello").unwrap().zero_width);
    }

    #[test]
    fn an_emote_without_files_is_dropped() {
        let set = parse_global(&fixture("global.json")).unwrap();
        assert!(set.get("Pending").is_none(), "no files yet");
        assert!(set.get("NoHost").is_none(), "no host yet");
    }

    #[test]
    fn a_foreign_or_plain_http_host_is_rejected() {
        let set = parse_global(&fixture("global.json")).unwrap();
        assert!(set.get("Evil").is_none(), "another host");
        assert!(set.get("Plain").is_none(), "http");
        assert!(set.get("Weird").is_none(), "not an emote folder");
    }

    #[test]
    fn the_emote_carries_what_the_card_needs() {
        let set = parse_global(&fixture("global.json")).unwrap();
        let hello = set.get("Hello").unwrap();
        assert_eq!(hello.id, format!("01HELLO0000000000000000000.{KEY}"));
        assert_eq!(emote_id(&hello.id), "01HELLO0000000000000000000");
        assert_eq!(hello.size, Some((32, 32)));
        assert!(!hello.animated);
        assert_eq!(hello.owner.as_deref(), Some("Kappa Dev"));
        assert_eq!(hello.owner_id.as_deref(), Some("01OWNER"));
        assert_eq!(hello.added_by.as_deref(), Some("01OWNER"));
        let wave = set.get("wave").unwrap();
        assert!(wave.animated);
        assert_eq!(wave.size, Some((64, 32)));
        assert!(wave.nsfw);
    }

    #[test]
    fn channel_reads_the_active_set() {
        let set = parse_channel("123", &fixture("channel.json")).unwrap().unwrap();
        assert_eq!(set.id, "cyextv-123");
        assert_eq!(set.scope, Scope::Channel);
        assert_eq!(set.name.as_deref(), Some("Main"));
        assert_eq!(set.emotes.len(), 1);
    }

    #[test]
    fn no_channel_or_no_active_set_is_none() {
        // 404 not_found is `null` from `channel_raw`.
        assert_eq!(parse_channel("1", &Value::Null).unwrap(), None);
        assert_eq!(parse_channel("1", &fixture("channel_no_set.json")).unwrap(), None);
        assert!(parse_channel("1", &json!([1])).is_err());
    }

    #[test]
    fn personal_sets_come_by_user() {
        let got = parse_personal(&fixture("personal.json")).unwrap();
        assert_eq!(got.len(), 1, "a set without any usable emote is left out");
        let (user, set) = &got[0];
        assert_eq!(user, "4001");
        assert_eq!(set.scope, Scope::Personal);
        assert_eq!(set.id, "cyextv-01PERSONALSET00000000000");
        assert!(set.get("mine").is_some());
        assert!(parse_personal(&json!({ "sets": [] })).unwrap().is_empty());
        assert!(parse_personal(&json!({})).unwrap().is_empty());
    }

    #[test]
    fn picture_addresses_are_checked() {
        assert_eq!(
            picture_key(&format!("https://cdn.cyex.tv/emotes/{KEY}")),
            Some(KEY)
        );
        for bad in [
            "http://cdn.cyex.tv/emotes/abc",
            "https://cdn.cyex.tv.evil.test/emotes/abc",
            "https://evil.test/emotes/abc",
            "https://cdn.cyex.tv@evil.test/emotes/abc",
            "https://cdn.cyex.tv/cosmetics/abc",
            "https://cdn.cyex.tv/emotes/",
            "https://cdn.cyex.tv/emotes/a/b",
            "https://cdn.cyex.tv/emotes/..",
            "https://cdn.cyex.tv/emotes/a?b",
            "",
        ] {
            assert_eq!(picture_key(bad), None, "{bad}");
        }
    }

    #[test]
    fn image_url_joins_host_and_file() {
        let host = format!("https://cdn.cyex.tv/emotes/{KEY}");
        assert_eq!(image_url(&host, "2x.webp").unwrap(), format!("{host}/2x.webp"));
        assert_eq!(
            image_url(&host, "1x_static.webp").unwrap(),
            format!("{host}/1x_static.webp")
        );
        assert_eq!(
            image_url("https://api.cyex.tv/x", "a.png").unwrap(),
            "https://api.cyex.tv/x/a.png"
        );
        assert_eq!(image_url("http://cdn.cyex.tv/emotes/a", "1x.webp"), None);
        assert_eq!(image_url("https://evil.test/emotes/a", "1x.webp"), None);
        assert_eq!(image_url("https://cdn.cyex.tv.evil.test/a", "1x.webp"), None);
        assert_eq!(image_url(&host, "../x.webp"), None);
        assert_eq!(image_url(&host, "a/b.webp"), None);
        assert_eq!(image_url(&host, ""), None);
    }
}
