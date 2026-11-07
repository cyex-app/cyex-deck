//! V108-C25 A link's hover card (1.52): one shape for every kind of link. Twitch clips,
//! VODs and channels come from our own lookups; everything else from Chatterino's
//! resolver (D8). Numbers and dates stay raw here; the UI formats them (3.1, 3.13).

use serde::Serialize;
use ts_rs::TS;

use crate::gql::{ChannelInfo, Clip, Video};
use crate::resolver::{LinkInfo, LinkText};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PreviewKind {
    Clip,
    Video,
    Channel,
    /// Anything the resolver knows.
    Web,
}

/// One fact on a card, formatted by the UI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
#[ts(export)]
pub enum PreviewFact {
    Views {
        n: u32,
    },
    Viewers {
        n: u32,
    },
    /// Seconds.
    Length {
        secs: u32,
    },
    /// RFC 3339.
    Date {
        at: String,
    },
    Game {
        name: String,
    },
    ClippedBy {
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LinkPreview {
    pub kind: PreviewKind,
    /// "Twitch", or the host the link ends up on.
    pub site: String,
    pub title: Option<String>,
    /// Under the title: the channel for Twitch links.
    pub subtitle: Option<String>,
    pub facts: Vec<PreviewFact>,
    /// The resolver's other lines; an empty line is a divider.
    pub lines: Vec<Vec<LinkText>>,
    /// https only.
    pub thumbnail: Option<String>,
    /// A channel's picture, drawn small and round.
    pub avatar: Option<String>,
    pub live: bool,
    /// The page says it's NSFW: the picture is blurred.
    pub nsfw: bool,
    /// Where it ends up after redirects, when known.
    pub final_url: Option<String>,
}

const TWITCH: &str = "Twitch";

fn https(url: Option<String>) -> Option<String> {
    url.filter(|u| u.starts_with("https://") && !u.contains(char::is_whitespace))
}

fn some(s: String) -> Option<String> {
    let s = s.trim().to_owned();
    (!s.is_empty()).then_some(s)
}

fn twitch(kind: PreviewKind) -> LinkPreview {
    LinkPreview {
        kind,
        site: TWITCH.into(),
        title: None,
        subtitle: None,
        facts: Vec::new(),
        lines: Vec::new(),
        thumbnail: None,
        avatar: None,
        live: false,
        nsfw: false,
        final_url: None,
    }
}

pub fn from_clip(c: Clip) -> LinkPreview {
    let mut facts = vec![
        PreviewFact::Views { n: c.views },
        PreviewFact::Length { secs: c.duration },
    ];
    if !c.created_at.is_empty() {
        facts.push(PreviewFact::Date { at: c.created_at });
    }
    if let Some(name) = c.game {
        facts.push(PreviewFact::Game { name });
    }
    if let Some(name) = c.clipped_by {
        facts.push(PreviewFact::ClippedBy { name });
    }
    LinkPreview {
        title: some(c.title),
        subtitle: some(c.channel_name),
        facts,
        thumbnail: https(c.thumbnail),
        ..twitch(PreviewKind::Clip)
    }
}

pub fn from_video(v: Video) -> LinkPreview {
    let mut facts = vec![
        PreviewFact::Length { secs: v.length },
        PreviewFact::Views { n: v.views },
    ];
    if !v.published_at.is_empty() {
        facts.push(PreviewFact::Date { at: v.published_at });
    }
    if let Some(name) = v.game {
        facts.push(PreviewFact::Game { name });
    }
    LinkPreview {
        title: some(v.title),
        subtitle: some(v.channel_name),
        facts,
        // A VOD still being processed has a placeholder picture.
        thumbnail: https(v.thumbnail).filter(|t| !t.contains("404_processing")),
        live: v.recording,
        ..twitch(PreviewKind::Video)
    }
}

/// The live picture Twitch's directory shows.
fn live_shot(login: &str) -> String {
    format!("https://static-cdn.jtvnw.net/previews-ttv/live_user_{login}-440x248.jpg")
}

pub fn from_channel(c: ChannelInfo) -> LinkPreview {
    let avatar = https(c.avatar);
    match c.stream {
        Some(s) => {
            let mut facts = vec![PreviewFact::Viewers { n: s.viewers }];
            if let Some(name) = s.game {
                facts.push(PreviewFact::Game { name });
            }
            LinkPreview {
                title: some(c.name),
                subtitle: some(s.title),
                facts,
                thumbnail: Some(live_shot(&c.login)),
                avatar,
                live: true,
                ..twitch(PreviewKind::Channel)
            }
        }
        None => LinkPreview {
            title: some(c.name),
            thumbnail: https(c.offline_image),
            avatar,
            ..twitch(PreviewKind::Channel)
        },
    }
}

/// The resolver's lines as a card: its first line is the title, the rest stay lines.
/// `None` when it has nothing to show.
pub fn from_resolver(url: &str, info: LinkInfo) -> Option<LinkPreview> {
    let mut lines = info.lines.into_iter();
    let title = lines
        .next()
        .map(|l| l.into_iter().map(|p| p.text).collect::<String>())
        .and_then(some);
    let mut rest: Vec<Vec<LinkText>> = lines.skip_while(Vec::is_empty).collect();
    while rest.last().is_some_and(Vec::is_empty) {
        rest.pop();
    }
    let nsfw = title.as_deref().is_some_and(says_nsfw) || rest.iter().flatten().any(|p| says_nsfw(&p.text));
    if title.is_none() && rest.is_empty() && info.thumbnail.is_none() {
        return None;
    }
    let site = host(info.link.as_deref().unwrap_or(url)).unwrap_or_default();
    Some(LinkPreview {
        kind: PreviewKind::Web,
        site,
        title,
        subtitle: None,
        facts: Vec::new(),
        lines: rest,
        thumbnail: https(info.thumbnail),
        avatar: None,
        live: false,
        nsfw,
        final_url: info.link,
    })
}

/// `NSFW` as a word, in capitals: how the resolver's pages flag it.
fn says_nsfw(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w == "NSFW")
}

/// `www.example.com` → `example.com`.
pub fn host(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    Some(u.host_str()?.trim_start_matches("www.").to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gql::{StreamInfo, VideoKind};

    fn text(t: &str, bold: bool) -> LinkText {
        LinkText { text: t.into(), bold }
    }

    fn clip() -> Clip {
        Clip {
            slug: "Funny-AbC".into(),
            title: " nope ".into(),
            views: 45_524,
            created_at: "2026-10-01T12:00:00Z".into(),
            duration: 30,
            thumbnail: Some("https://clips-media/t.jpg".into()),
            channel_login: "dexm".into(),
            channel_name: "DexM".into(),
            clipped_by: Some("Someone".into()),
            game: None,
            edit_restricted: None,
        }
    }

    #[test]
    fn clips() {
        let p = from_clip(clip());
        assert_eq!(p.kind, PreviewKind::Clip);
        assert_eq!(p.site, "Twitch");
        assert_eq!(p.title.as_deref(), Some("nope"));
        assert_eq!(p.subtitle.as_deref(), Some("DexM"));
        assert_eq!(
            p.facts,
            [
                PreviewFact::Views { n: 45_524 },
                PreviewFact::Length { secs: 30 },
                PreviewFact::Date {
                    at: "2026-10-01T12:00:00Z".into()
                },
                PreviewFact::ClippedBy {
                    name: "Someone".into()
                },
            ]
        );
        assert_eq!(p.thumbnail.as_deref(), Some("https://clips-media/t.jpg"));
        let p = from_clip(Clip {
            thumbnail: Some("http://plain".into()),
            title: String::new(),
            ..clip()
        });
        assert!(p.thumbnail.is_none() && p.title.is_none());
    }

    fn video() -> Video {
        Video {
            id: "12".into(),
            title: "Long stream".into(),
            length: 7_384,
            published_at: "2026-09-30T20:00:00Z".into(),
            views: 1_234_567,
            kind: VideoKind::Archive,
            recording: false,
            restricted: false,
            thumbnail: Some("https://vod/t-320x180.jpg".into()),
            channel_login: "a".into(),
            channel_name: "A".into(),
            game: Some("Chess".into()),
        }
    }

    #[test]
    fn videos() {
        let p = from_video(video());
        assert_eq!(p.kind, PreviewKind::Video);
        assert_eq!(p.facts[0], PreviewFact::Length { secs: 7_384 });
        assert!(p.facts.contains(&PreviewFact::Game { name: "Chess".into() }));
        assert!(!p.live);
        let p = from_video(Video {
            recording: true,
            thumbnail: Some("https://vod-secure.twitch.tv/_404/404_processing_320x180.png".into()),
            ..video()
        });
        assert!(p.live && p.thumbnail.is_none());
    }

    fn channel(stream: Option<StreamInfo>) -> ChannelInfo {
        ChannelInfo {
            id: "1".into(),
            login: "lirik".into(),
            name: "LIRIK".into(),
            avatar: Some("https://avatar".into()),
            offline_image: Some("https://offline".into()),
            stream,
        }
    }

    #[test]
    fn channels() {
        let off = from_channel(channel(None));
        assert_eq!((off.kind, off.live), (PreviewKind::Channel, false));
        assert_eq!(off.title.as_deref(), Some("LIRIK"));
        assert_eq!(off.thumbnail.as_deref(), Some("https://offline"));
        assert_eq!(off.avatar.as_deref(), Some("https://avatar"));
        assert!(off.facts.is_empty());
        let on = from_channel(channel(Some(StreamInfo {
            rerun: false,
            title: "chill".into(),
            game: Some("Just Chatting".into()),
            viewers: 12_000,
            started_at: String::new(),
        })));
        assert!(on.live);
        assert_eq!(on.subtitle.as_deref(), Some("chill"));
        assert_eq!(
            on.facts,
            [
                PreviewFact::Viewers { n: 12_000 },
                PreviewFact::Game {
                    name: "Just Chatting".into()
                }
            ]
        );
        assert_eq!(
            on.thumbnail.as_deref(),
            Some("https://static-cdn.jtvnw.net/previews-ttv/live_user_lirik-440x248.jpg")
        );
    }

    #[test]
    fn resolver_cards() {
        let info = LinkInfo {
            link: Some("https://www.youtube.com/watch?v=x".into()),
            lines: vec![
                vec![text("A video", true)],
                vec![],
                vec![text("Channel:", true), text(" Someone", false)],
                vec![],
            ],
            thumbnail: Some("https://i.ytimg.com/x.jpg".into()),
        };
        let p = from_resolver("https://youtu.be/x", info).unwrap();
        assert_eq!(p.kind, PreviewKind::Web);
        assert_eq!(p.site, "youtube.com");
        assert_eq!(p.title.as_deref(), Some("A video"));
        assert_eq!(p.lines.len(), 1);
        assert!(!p.nsfw);
        assert_eq!(p.final_url.as_deref(), Some("https://www.youtube.com/watch?v=x"));

        let nsfw = LinkInfo {
            link: None,
            lines: vec![vec![text("Pic (NSFW)", false)]],
            thumbnail: None,
        };
        let p = from_resolver("https://www.imgur.com/a", nsfw).unwrap();
        assert!(p.nsfw);
        assert_eq!(p.site, "imgur.com");
        assert!(!says_nsfw("unsfwx nsfw"));

        assert!(from_resolver("https://x.com", LinkInfo::default()).is_none());
    }
}
