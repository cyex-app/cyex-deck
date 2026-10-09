//! Twitch links that open in the app instead of the browser: clips (1.19, 1.20) and
//! VODs (P7), with the time a VOD link starts at.

use url::Url;

use crate::model::LinkTarget;

pub fn target(url: &Url) -> Option<LinkTarget> {
    let host = url.host_str()?.trim_start_matches("www.");
    let parts: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let query = |key: &str| url.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned());
    match (host, parts.as_slice()) {
        ("clips.twitch.tv", ["embed"]) => clip(query("clip")?),
        ("clips.twitch.tv", [slug]) if *slug != "create" => clip((*slug).to_owned()),
        ("twitch.tv" | "m.twitch.tv", [_, "clip", slug] | ["clip", slug]) => clip((*slug).to_owned()),
        ("twitch.tv" | "m.twitch.tv", ["videos", id] | [_, "video", id] | [_, "v", id]) => video(id, query("t")),
        ("player.twitch.tv", []) => video(query("video")?.trim_start_matches('v'), query("t")),
        _ => None,
    }
}

fn clip(slug: String) -> Option<LinkTarget> {
    let valid = slug.len() <= 120 && slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    (valid && !slug.is_empty()).then_some(LinkTarget::Clip { slug })
}

fn video(id: &str, t: Option<String>) -> Option<LinkTarget> {
    (!id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())).then(|| LinkTarget::Video {
        id: id.to_owned(),
        at: t.as_deref().map_or(0, seconds),
    })
}

/// `1h2m3s`, `90m`, `45s` or plain seconds.
fn seconds(t: &str) -> u32 {
    if let Ok(s) = t.parse() {
        return s;
    }
    let (mut total, mut n) = (0u32, 0u32);
    for c in t.chars() {
        match c {
            '0'..='9' => n = n.saturating_mul(10).saturating_add(c as u32 - '0' as u32),
            'h' => (total, n) = (total.saturating_add(n.saturating_mul(3600)), 0),
            'm' => (total, n) = (total.saturating_add(n.saturating_mul(60)), 0),
            's' => (total, n) = (total.saturating_add(n), 0),
            _ => return 0,
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(url: &str) -> Option<LinkTarget> {
        target(&Url::parse(url).unwrap())
    }

    #[test]
    fn clips() {
        let slug = LinkTarget::Clip {
            slug: "FunnyCatClip-AbC_12".into(),
        };
        for u in [
            "https://clips.twitch.tv/FunnyCatClip-AbC_12",
            "https://clips.twitch.tv/embed?clip=FunnyCatClip-AbC_12&parent=x",
            "https://www.twitch.tv/xqc/clip/FunnyCatClip-AbC_12?filter=clips",
            "https://m.twitch.tv/clip/FunnyCatClip-AbC_12",
        ] {
            assert_eq!(of(u).as_ref(), Some(&slug), "{u}");
        }
        for u in ["https://clips.twitch.tv/create", "https://twitch.tv/xqc", "https://x.com/a/clip/b"] {
            assert_eq!(of(u), None, "{u}");
        }
    }

    #[test]
    fn videos_with_their_start() {
        let v = |id: &str, at| Some(LinkTarget::Video { id: id.into(), at });
        assert_eq!(of("https://www.twitch.tv/videos/2895316391"), v("2895316391", 0));
        assert_eq!(of("https://twitch.tv/videos/12?t=1h2m3s"), v("12", 3723));
        assert_eq!(of("https://m.twitch.tv/videos/12?t=90"), v("12", 90));
        assert_eq!(of("https://player.twitch.tv/?video=v12&t=2m&parent=x"), v("12", 120));
        assert_eq!(of("https://twitch.tv/videos/abc"), None);
        assert_eq!(seconds("5x"), 0);
    }
}
