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
