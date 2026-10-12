use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// Playback (1.1, 1.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Player {
    /// 1.2
    pub block_ads: bool,
    /// Twitch's low latency mode: a few seconds behind the stream instead of ~10.
    pub low_latency: bool,
    /// What a player starts in. A player's own menu only changes that player.
    pub quality: Quality,
    /// P4, V101-25 Where the small stream sits while the channel's pages show.
    pub mini_corner: Corner,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            block_ads: true,
            low_latency: true,
            quality: Quality::Auto,
            mini_corner: Corner::BottomRight,
        }
    }
}

impl Schema for Player {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Quality {
    /// The best that fits the player on screen.
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "source")]
    Source,
    #[serde(rename = "1080")]
    P1080,
    #[serde(rename = "720")]
    P720,
    #[serde(rename = "480")]
    P480,
    #[serde(rename = "360")]
    P360,
    #[serde(rename = "160")]
    P160,
}

/// A corner of the player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualities_read_as_plain_names() {
        let p: Player = serde_json::from_str(r#"{"quality":"720","blockAds":false}"#).unwrap();
        assert_eq!((p.quality, p.block_ads, p.low_latency), (Quality::P720, false, true));
        assert_eq!(serde_json::to_value(Quality::Auto).unwrap(), "auto");
        assert_eq!(p.mini_corner, Corner::BottomRight);
        let p: Player = serde_json::from_str(r#"{"miniCorner":"topLeft"}"#).unwrap();
        assert_eq!(p.mini_corner, Corner::TopLeft);
    }
}
