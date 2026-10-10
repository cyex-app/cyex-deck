//! Pieces of a message body, tokenized in Rust so the UI only renders (ARCHITECTURE §2).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
#[ts(export)]
pub enum Fragment {
    Text {
        text: String,
    },
    /// Image URLs are built in the UI from provider + id. The extras are left out for
    /// plain emotes.
    Emote {
        provider: Provider,
        id: String,
        name: String,
        /// C7 Zero-width emotes drawn on top, in order.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        stack: Option<Vec<Layer>>,
        /// C7 FFZ modifier flags ([`mods`]), BTTV's mapped onto them.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        mods: Option<u32>,
        /// 2.4 Unlisted 7TV emote.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        unlisted: Option<bool>,
        #[serde(flatten)]
        look: EmoteLook,
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

/// How a provider emote's picture is drawn. Twitch emotes leave it empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmoteLook {
    /// E2 Has a still frame to show while the window isn't focused.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub animated: Option<bool>,
    /// Size at 1x in px, when the provider says, so lines don't jump as pictures load.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub size: Option<(u16, u16)>,
}

/// C7 A zero-width emote on top of another.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Layer {
    pub provider: Provider,
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub look: EmoteLook,
}

/// C7 Modifier bits, as FFZ numbers them (`modifier_flags`).
pub mod mods {
    pub const HIDDEN: u32 = 1;
    pub const FLIP_X: u32 = 1 << 1;
    pub const FLIP_Y: u32 = 1 << 2;
    pub const GROW_X: u32 = 1 << 3;
    pub const SLIDE: u32 = 1 << 4;
    pub const APPEAR: u32 = 1 << 5;
    pub const LEAVE: u32 = 1 << 6;
    pub const ROTATE: u32 = 1 << 7;
    pub const ROTATE_90: u32 = 1 << 8;
    pub const GREYSCALE: u32 = 1 << 9;
    pub const SEPIA: u32 = 1 << 10;
    pub const RAINBOW: u32 = 1 << 11;
    pub const HYPER_RED: u32 = 1 << 12;
    pub const SHAKE: u32 = 1 << 13;
    pub const CURSED: u32 = 1 << 14;
    pub const JAM: u32 = 1 << 15;
    pub const BOUNCE: u32 = 1 << 16;
    pub const NO_SPACE: u32 = 1 << 17;
    /// BTTV's own: `w!` four times as wide.
    pub const WIDE: u32 = 1 << 18;
    /// BTTV's own: `l!` turned left.
    pub const ROTATE_LEFT: u32 = 1 << 19;
    /// BTTV's own: `p!` cycling colors.
    pub const PARTY: u32 = 1 << 20;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
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

    /// A plain emote, e.g. one Twitch tagged.
    pub fn emote(provider: Provider, id: impl Into<String>, name: impl Into<String>) -> Self {
        Fragment::Emote {
            provider,
            id: id.into(),
            name: name.into(),
            stack: None,
            mods: None,
            unlisted: None,
            look: EmoteLook::default(),
        }
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
