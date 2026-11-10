//! V110-U70 Link cards from our own resolver (embed.cyex.app, a Cloudflare Worker): the Worker
//! fetches the link and its pictures, so neither the link's host nor an image host ever sees
//! this machine. Trusted links ask it first; Chatterino's resolver (D8) is the fallback.
//! Its answer is data only; every field is cleaned again here, and a picture that is not the
//! Worker's own `/v1/img` form is dropped, never loaded direct (V110-F75).

use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::preview::{self, LinkPreview, PreviewFact, PreviewKind};
use crate::resolver::LinkText;
use crate::{Http, Result};

const BASE: &str = "https://embed.cyex.app";
/// Past this the Worker counts as down and the fallback answers.
const TIMEOUT: Duration = Duration::from_secs(4);
/// The Worker caps its own cards far below this.
const MAX_BODY: usize = 256 * 1024;
const TITLE_MAX: usize = 200;
const TEXT_MAX: usize = 400;

/// The Worker; in mocks builds `CYEX_EMBED_MOCK=http://127.0.0.1:<port>` stands in.
fn base() -> String {
    #[cfg(any(test, feature = "mocks"))]
    if let Ok(u) = std::env::var("CYEX_EMBED_MOCK")
        && crate::resolver::mock_ok(&u)
    {
        return u.trim_end_matches('/').to_owned();
    }
    BASE.to_owned()
}

/// V110-U70 A picture served by the Worker's `/v1/img`: the only direct kind of picture a Worker
/// card may carry.
pub fn is_img(url: &str) -> bool {
    url.strip_prefix(&format!("{}/v1/img?u=", base()))
        .is_some_and(|r| !r.is_empty() && !r.contains(char::is_whitespace))
}

/// What the Worker said, and so what happens next.
#[derive(Debug, PartialEq)]
pub enum Next {
    /// A card to show.
    Use(Box<LinkPreview>),
    /// "Nothing to show" (404): no fallback.
    Nothing,
    /// Timeout, any other status, bad JSON or an empty card: ask Chatterino's resolver.
    Fallback,
}

/// The decision, pure: `status` is `None` when the Worker could not be reached in time.
pub fn decide(status: Option<u16>, body: &str) -> Next {
    match status {
        Some(200) => match card(body) {
            Some(p) => Next::Use(Box::new(p)),
            None => Next::Fallback,
        },
        Some(404) => Next::Nothing,
        _ => Next::Fallback,
    }
}

/// Ask the Worker for `url`'s card.
pub async fn lookup(http: &Http, url: &str) -> Next {
    match fetch(http, url).await {
        Ok((status, body)) => decide(Some(status), &body),
        Err(_) => decide(None, ""),
    }
}

async fn fetch(http: &Http, url: &str) -> Result<(u16, String)> {
    let encoded: String = url::form_urlencoded::byte_serialize(url.as_bytes()).collect();
    let mut res = http
        .0
        .get(format!("{}/v1/card?url={encoded}", base()))
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|e| crate::Error::Network(e.without_url()))?;
    let status = res.status().as_u16();
    if status != 200 {
        return Ok((status, String::new()));
    }
    if res.content_length().is_some_and(|n| n > MAX_BODY as u64) {
        return Ok((502, String::new()));
    }
    // V110-F85 A running limit: a chunked body can't skip the Content-Length check above.
    let mut bytes = Vec::new();
    while let Some(chunk) = res.chunk().await.map_err(|e| crate::Error::Network(e.without_url()))? {
        if !crate::http::push_within(&mut bytes, &chunk, MAX_BODY) {
            return Ok((502, String::new()));
        }
    }
    Ok((status, String::from_utf8_lossy(&bytes).into_owned()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Card {
    v: Option<u32>,
    site: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    text: Option<String>,
    #[serde(default)]
    facts: Vec<Value>,
    thumbnail: Option<String>,
    avatar: Option<String>,
    #[serde(default)]
    nsfw: bool,
    final_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
enum Fact {
    Length { secs: u64 },
    Views { n: u64 },
    Likes { n: u64 },
    Date { at: String },
}

/// Plain text: no control characters, runs of space collapsed, at most `max` characters.
fn clean(s: Option<String>, max: usize) -> Option<String> {
    let s = s?;
    let mut out = String::new();
    for w in s.split(|c: char| c.is_whitespace() || c.is_control()).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
    }
    let out: String = out.chars().take(max).collect();
    let out = out.trim_end().to_owned();
    (!out.is_empty()).then_some(out)
}

fn fact(v: Value) -> Option<PreviewFact> {
    let n = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
    Some(match serde_json::from_value::<Fact>(v).ok()? {
        Fact::Length { secs } => PreviewFact::Length { secs: n(secs) },
        Fact::Views { n: v } => PreviewFact::Views { n: n(v) },
        Fact::Likes { n: v } => PreviewFact::Likes { n: n(v) },
        Fact::Date { at } => {
            let at = at.trim();
            if at.is_empty() || at.len() > 40 || at.contains(char::is_control) {
                return None;
            }
            PreviewFact::Date { at: at.to_owned() }
        }
    })
}

fn https(u: Option<String>) -> Option<String> {
    u.filter(|u| u.starts_with("https://") && !u.contains(char::is_whitespace))
}

/// The Worker's card as a preview. `None` when it is not a v1 card or has nothing to show.
/// Kinds web, video, image and post all draw as a web card.
pub fn card(body: &str) -> Option<LinkPreview> {
    let c: Card = serde_json::from_str(body).ok()?;
    if c.v != Some(1) {
        return None;
    }
    let title = clean(c.title, TITLE_MAX);
    let subtitle = clean(c.subtitle, TITLE_MAX);
    let text = clean(c.text, TEXT_MAX);
    let facts: Vec<PreviewFact> = c.facts.into_iter().filter_map(fact).collect();
    // V110-F75 A picture that is not the Worker's own is dropped, not loaded.
    let thumbnail = c.thumbnail.filter(|t| is_img(t));
    let avatar = c.avatar.filter(|t| is_img(t));
    if title.is_none() && subtitle.is_none() && text.is_none() && facts.is_empty() && thumbnail.is_none() {
        return None;
    }
    let final_url = https(c.final_url);
    let site = clean(c.site, TITLE_MAX)
        .or_else(|| final_url.as_deref().and_then(preview::host))
        .unwrap_or_default();
    Some(LinkPreview {
        kind: PreviewKind::Web,
        site,
        title,
        subtitle,
        facts,
        // The description is one plain line under the facts.
        lines: text.into_iter().map(|t| vec![LinkText { text: t, bold: false }]).collect(),
        thumbnail,
        avatar,
        live: false,
        nsfw: c.nsfw,
        final_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMG: &str = "https://embed.cyex.app/v1/img?u=https%3A%2F%2Fi.ytimg.com%2Fvi%2Fa%2Fhqdefault.jpg";

    fn body(extra: &str) -> String {
        format!(r#"{{"v":1,"kind":"video","site":"YouTube",{extra}}}"#)
    }

    #[test]
    fn a_full_card() {
        let b = body(&format!(
            r#""title":"  A  video\n","subtitle":"Someone","text":"It is\u0007 about\n things.","facts":[{{"t":"length","secs":214}},{{"t":"views","n":123}},{{"t":"date","at":"2026-11-09T10:00:00Z"}},{{"t":"likes","n":5}},{{"t":"shares","n":1}},{{"t":"likes"}}],"thumbnail":"{IMG}","avatar":"{IMG}","nsfw":true,"finalUrl":"https://www.youtube.com/watch?v=a""#
        ));
        let p = card(&b).unwrap();
        assert_eq!(p.kind, PreviewKind::Web);
        assert_eq!(p.site, "YouTube");
        assert_eq!(p.title.as_deref(), Some("A video"));
        assert_eq!(p.subtitle.as_deref(), Some("Someone"));
        assert_eq!(
            p.facts,
            [
                PreviewFact::Length { secs: 214 },
                PreviewFact::Views { n: 123 },
                PreviewFact::Date {
                    at: "2026-11-09T10:00:00Z".into()
                },
                PreviewFact::Likes { n: 5 },
            ]
        );
        assert_eq!(p.lines.len(), 1);
        assert_eq!(p.lines[0][0].text, "It is about things.");
        assert_eq!(p.thumbnail.as_deref(), Some(IMG));
        assert_eq!(p.avatar.as_deref(), Some(IMG));
        assert!(p.nsfw && !p.live);
        assert_eq!(p.final_url.as_deref(), Some("https://www.youtube.com/watch?v=a"));
    }

    #[test]
    fn small_cards_and_limits() {
        let p = card(&body(r#""title":"x""#)).unwrap();
        assert!(p.facts.is_empty() && p.lines.is_empty() && p.thumbnail.is_none() && !p.nsfw);
        // No site: the final link's host.
        let p = card(r#"{"v":1,"title":"x","finalUrl":"https://www.reddit.com/r/a"}"#).unwrap();
        assert_eq!(p.site, "reddit.com");
        // Huge counts clamp; long text is cut.
        let long = "a".repeat(900);
        let p = card(&body(&format!(r#""title":"{long}","text":"{long}","facts":[{{"t":"views","n":99999999999}}]"#))).unwrap();
        assert_eq!(p.title.unwrap().chars().count(), TITLE_MAX);
        assert_eq!(p.lines[0][0].text.chars().count(), TEXT_MAX);
        assert_eq!(p.facts, [PreviewFact::Views { n: u32::MAX }]);
        // A non-https final link is not kept.
        let p = card(r#"{"v":1,"title":"x","finalUrl":"http://a.b/"}"#).unwrap();
        assert!(p.final_url.is_none());
    }

    #[test]
    fn bad_or_empty_cards_are_nothing() {
        for b in [
            "",
            "not json",
            "[]",
            r#"{"title":"x"}"#,
            r#"{"v":2,"title":"x"}"#,
            r#"{"v":1}"#,
            r#"{"v":1,"title":"   ","text":"\n","facts":[{"t":"shares","n":1}]}"#,
            r#"{"v":1,"title":1}"#,
        ] {
            assert!(card(b).is_none(), "{b}");
        }
    }

    #[test]
    fn v110_u70_only_the_workers_pictures_are_kept() {
        for direct in [
            "https://i.ytimg.com/vi/a/hqdefault.jpg",
            "https://braize.pajlada.com/chatterino/thumbnail/x",
            "http://embed.cyex.app/v1/img?u=x",
            "https://embed.cyex.app/v1/img",
            "https://embed.cyex.app/v1/img?u=",
            "https://embed.cyex.app/v1/card?url=x",
            "https://embed.cyex.app.evil.io/v1/img?u=x",
            "https://embed.cyex.app@evil.io/v1/img?u=x",
            "https://evil.io/https://embed.cyex.app/v1/img?u=x",
            "https://embed.cyex.app/v1/img?u=a b",
            "data:image/png;base64,AA",
            "",
        ] {
            assert!(!is_img(direct), "{direct}");
            let b = body(&format!(r#""title":"x","thumbnail":"{direct}","avatar":"{direct}""#));
            let p = card(&b).unwrap();
            assert!(p.thumbnail.is_none() && p.avatar.is_none(), "{direct}");
        }
        assert!(is_img(IMG));
        // A card with only a foreign picture has nothing left to show.
        assert!(card(&body(r#""thumbnail":"https://i.ytimg.com/a.jpg""#)).is_none());
        // A card with only a Worker picture is enough.
        assert!(card(&body(&format!(r#""thumbnail":"{IMG}""#))).is_some());
    }

    #[test]
    fn v110_u70_what_happens_next() {
        let ok = body(r#""title":"x""#);
        assert!(matches!(decide(Some(200), &ok), Next::Use(_)));
        // Bad JSON or an empty card on a 200: the fallback.
        assert_eq!(decide(Some(200), "nope"), Next::Fallback);
        assert_eq!(decide(Some(200), r#"{"v":1}"#), Next::Fallback);
        // The Worker says "nothing": stop.
        assert_eq!(decide(Some(404), r#"{"error":"none"}"#), Next::Nothing);
        // Everything else, and no answer in time: the fallback.
        for s in [400, 403, 429, 500, 502, 503, 204, 301] {
            assert_eq!(decide(Some(s), &ok), Next::Fallback, "{s}");
        }
        assert_eq!(decide(None, ""), Next::Fallback);
    }
}
