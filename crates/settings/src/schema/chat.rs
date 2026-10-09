use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// Twitch's longest timeout: 14 days.
const MAX_TIMEOUT_SECS: u32 = 1_209_600;
const MAX_TIMEOUT_PRESETS: usize = 8;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Chat {
    /// 2.3 Badges per provider, each its own toggle.
    pub badges: BadgeToggles,
    /// 2.3 Name paints per provider.
    pub paints: PaintToggles,
    /// 2.3
    pub twitch_emotes: bool,
    /// 2.3 7TV, BTTV and FFZ emotes as one group.
    pub third_party_emotes: bool,
    /// 2.4
    pub unlisted_emotes: UnlistedEmotes,
    /// 2.5
    pub gifs: GifMode,
    /// 2.10
    pub timestamps: Timestamps,
    /// 2.11 Quick timeout durations in seconds, shortest first.
    pub timeout_presets: Vec<u32>,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            badges: BadgeToggles::default(),
            paints: PaintToggles::default(),
            twitch_emotes: true,
            third_party_emotes: true,
            unlisted_emotes: UnlistedEmotes::Blur,
            gifs: GifMode::Show,
            timestamps: Timestamps::default(),
            timeout_presets: vec![60, 600, 3600, 86_400],
        }
    }
}

impl Schema for Chat {
    fn sanitize(&mut self) {
        let presets = &mut self.timeout_presets;
        presets.retain(|s| (1..=MAX_TIMEOUT_SECS).contains(s));
        presets.sort_unstable();
        presets.dedup();
        presets.truncate(MAX_TIMEOUT_PRESETS);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeToggles {
    pub twitch: bool,
    pub seventv: bool,
    pub bttv: bool,
    pub ffz: bool,
}

impl Default for BadgeToggles {
    fn default() -> Self {
        Self {
            twitch: true,
            seventv: true,
            bttv: true,
            ffz: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct PaintToggles {
    pub seventv: bool,
    pub bttv: bool,
}

impl Default for PaintToggles {
    fn default() -> Self {
        Self {
            seventv: true,
            bttv: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum UnlistedEmotes {
    Hide,
    #[default]
    Blur,
    Show,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GifMode {
    #[default]
    Show,
    Link,
    Text,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Timestamps {
    pub show: bool,
    pub seconds: bool,
    /// 24-hour by default (3.2).
    pub hour12: bool,
}

impl Default for Timestamps {
    fn default() -> Self {
        Self {
            show: true,
            seconds: false,
            hour12: false,
        }
    }
}
