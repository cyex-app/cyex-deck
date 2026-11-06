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

/// A badge the account owns: `founder` (with its number), `supporter`, `custom`.
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

    pub fn founder_number(&self) -> Option<u32> {
        self.badges.iter().find(|b| b.id == "founder").and_then(|b| b.number)
    }
}

/// V109-B06 One Supporter+ upload in the history (`GET /plus/badges`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct CyexUpload {
    #[ts(type = "number")]
    pub id: u64,
    /// Only approved pictures are public; the others' owner previews need the website's
    /// sign-in, so they come through as `None`; `None` too once the files are purged.
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
    /// Drops pictures the app can't load (anything not approved) and keeps 20 at most.
    pub fn tidy(mut self) -> Self {
        self.items.truncate(20);
        for u in &mut self.items {
            if u.status != "approved" {
                u.images = None;
                u.restorable = false;
            }
        }
        self
    }
}

/// The cosmetics area of one account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum CyexView {
    /// With the built-in badges' pictures for the preview.
    Ready {
        me: Box<CyexMe>,
        founder: CyexPics,
        supporter: CyexPics,
    },
    /// Offline, or the server is down.
    Offline,
    /// The account has no Twitch token the server takes.
    NoToken,
}
