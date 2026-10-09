//! Pieces of a message body, tokenized in Rust so the UI only renders (ARCHITECTURE §2).

use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
#[ts(export)]
pub enum Fragment {
    Text {
        text: String,
    },
    /// Image URLs are built in the UI from provider + id.
    Emote {
        provider: Provider,
        id: String,
        name: String,
    },
    /// `@name`. Clicking it opens the usercard.
    Mention {
        text: String,
        login: String,
    },
    /// X2: `host` is the real domain (IDN shown as punycode), so look-alikes stand out.
    Link {
        text: String,
        url: String,
        host: String,
        /// Twitch clips and VODs open in the app.
        target: Option<LinkTarget>,
    },
    /// Bits: `cheer100`.
    Cheer {
        prefix: String,
        amount: u32,
    },
    /// 1.43 Chat GIF. The URL must be used as sent.
    Gif {
        id: String,
        url: String,
    },
}

/// What a Twitch link opens in the app.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum LinkTarget {
    /// 1.19, 1.20
    Clip { slug: String },
    /// P7 `at`: seconds to start from.
    Video { id: String, at: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Provider {
    Twitch,
    Seventv,
    Bttv,
    Ffz,
}

impl Fragment {
    pub fn text(text: impl Into<String>) -> Self {
        Fragment::Text { text: text.into() }
    }

    /// The text this fragment came from, for copying and searching.
    pub fn source(&self) -> String {
        match self {
            Fragment::Text { text } | Fragment::Mention { text, .. } | Fragment::Link { text, .. } => {
                text.clone()
            }
            Fragment::Emote { name, .. } => name.clone(),
            Fragment::Cheer { prefix, amount } => format!("{prefix}{amount}"),
            Fragment::Gif { url, .. } => url.clone(),
        }
    }
}

/// Plain text of a body.
pub fn plain(body: &[Fragment]) -> String {
    body.iter().map(Fragment::source).collect()
}
