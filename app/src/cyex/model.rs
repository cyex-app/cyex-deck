//! V109-B02 What api.cyex.app answers for an account, as the UI gets it. Unknown fields are
//! ignored; missing ones fall back to empty.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CyexPics {
    #[serde(rename = "1x", default)]
    pub x1: String,
    #[serde(rename = "2x", default)]
    pub x2: String,
    #[serde(rename = "4x", default)]
    pub x4: String,
    /// 256 px, only on newer uploads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub large: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct CyexTwitch {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub avatar: Option<String>,
}

/// A badge the account owns: any one-time store badge (`founder`, `supporter`, …; numbered ones
/// with their number), or `custom`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct CyexOwned {
    pub id: String,
    pub number: Option<u32>,
}

/// The Supporter+ picture's review state (`plus.badge`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct CyexPlusBadge {
    /// `pending`, `approved`, `rejected`, … as the server says.
    pub status: String,
    /// V109-B06 Why it was rejected or removed (empty when the server gave none).
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct CyexPlus {
    pub active: bool,
    pub source: Option<String>,
    pub until: Option<String>,
    pub renews_at: Option<String>,
    pub badge: Option<CyexPlusBadge>,
    /// The approved picture showing now.
    pub live: Option<CyexPics>,
}

/// What the account wears. `custom` is the switch; `shown` what is visible now (≤ 2, in order).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct CyexEquipped {
    pub main: Option<String>,
    pub custom: bool,
    pub shown: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct CyexMe {
    pub twitch: CyexTwitch,
    pub badges: Vec<CyexOwned>,
    pub plus: CyexPlus,
    pub equipped: CyexEquipped,
}

impl CyexMe {
    /// A founder counts only with its number.
    pub fn owns(&self, id: &str) -> bool {
        self.badges
            .iter()
            .any(|b| b.id == id && (id != "founder" || b.number.is_some()))
    }

    /// V109-B10 The number of a numbered badge the account owns.
    pub fn number_of(&self, id: &str) -> Option<u32> {
        self.badges.iter().find(|b| b.id == id).and_then(|b| b.number)
    }
}

/// V109-B06 One Supporter+ upload in the history (`GET /plus/badges`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct CyexUpload {
    #[ts(type = "number")]
    pub id: u64,
    /// Approved: public. Others: owner previews (`/me/badge/…`), loaded through
    /// `cyex_badge_preview` (V109-B07). `None` once the files are purged.
    pub images: Option<CyexPics>,
    pub status: String,
    /// Rejected or removed: why.
    pub reason: Option<String>,
    /// Approved, not live, files kept: "Use again" brings it back.
    pub restorable: bool,
    pub uploaded_at: String,
    pub reviewed_at: Option<String>,
}

/// V109-B06 The account's uploads, newest first, and which one is live.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct CyexHistory {
    pub items: Vec<CyexUpload>,
    #[ts(type = "number | null")]
    pub live: Option<u64>,
}

impl CyexHistory {
    /// Keeps 20 at most; only approved uploads can be used again.
    pub fn tidy(mut self) -> Self {
        self.items.truncate(20);
        for u in &mut self.items {
            if u.status != "approved" {
                u.restorable = false;
            }
        }
        self
    }
}

/// V109-B10 One one-time badge of the catalog (store order), with its title and pictures.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CyexKind {
    pub id: String,
    pub title: String,
    pub images: CyexPics,
    /// V109-B12 The still frames when `images` move (official badges only).
    pub stills: Option<CyexPics>,
}

/// The cosmetics area of one account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum CyexView {
    /// With the catalog's one-time badges (titles, pictures) for the picker and the preview.
    Ready { me: Box<CyexMe>, kinds: Vec<CyexKind> },
    /// Offline, or the server is down.
    Offline,
    /// V109-B16 The account has no Helix token (Sign in with Twitch), only the code sign-in.
    NoToken,
    /// V109-B16 The server refused the account's Helix token.
    Refused,
}
