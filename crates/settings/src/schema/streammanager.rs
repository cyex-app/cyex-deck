use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// 1.35, X4 Stream manager options: the activity-feed filters and the OBS status files.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct StreamManager {
    /// Which activity kinds the feed shows.
    pub activity: ActivityFilters,
    /// X4 Status files for OBS.
    pub obs: Obs,
    /// D4 The channel's alerts were seen playing in Twitch's alert box, so the feed offers
    /// Replay and Skip. Learned, not set by hand.
    pub alert_box: bool,
    /// V102-22 Your chat next to the cards.
    pub chat: ManagerChat,
}

/// V102-22 The stream manager's chat panel: on the right by default, its width dragged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct ManagerChat {
    pub shown: bool,
    /// The chat's share of the width, 0.15 to 0.85.
    pub share: f32,
}

impl Default for ManagerChat {
    fn default() -> Self {
        Self { shown: true, share: 0.3 }
    }
}

/// Each activity kind the feed can show. All on until the user hides one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityFilters {
    pub follows: bool,
    pub subs: bool,
    pub gifts: bool,
    pub cheers: bool,
    pub raids: bool,
    pub redemptions: bool,
    /// Watch streaks; only a GQL sign-in gets them (D30).
    pub streaks: bool,
    /// D3 Power-ups, hype trains and goals; GQL sign-in only.
    pub power_ups: bool,
    pub hype_trains: bool,
    pub goals: bool,
}

impl Default for ActivityFilters {
    fn default() -> Self {
        Self {
            follows: true,
            subs: true,
            gifts: true,
            cheers: true,
            raids: true,
            redemptions: true,
            streaks: true,
            power_ups: true,
            hype_trains: true,
            goals: true,
        }
    }
}

/// X4 Writes the live title, category, viewer count and more to text files OBS can read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Obs {
    /// Off by default.
    pub enabled: bool,
    /// Folder for the files; empty uses `stream-status/` next to the settings folder.
    pub folder: String,
}

impl Schema for StreamManager {
    fn sanitize(&mut self) {
        let share = self.chat.share;
        self.chat.share = if share.is_finite() { share.clamp(0.15, 0.85) } else { ManagerChat::default().share };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_default_on() {
        let f = ActivityFilters::default();
        assert!(f.follows && f.subs && f.gifts && f.cheers && f.raids && f.redemptions && f.streaks);
        // Files saved before streaks existed get them on.
        let old: ActivityFilters = serde_json::from_str(r#"{"follows":false}"#).unwrap();
        assert!(!old.follows && old.streaks && old.power_ups && old.goals);
        assert!(!Obs::default().enabled);
    }

    #[test]
    fn chat_shows_by_default_and_keeps_a_sane_width() {
        let mut sm: StreamManager = serde_json::from_str(r#"{"chat":{"share":4}}"#).unwrap();
        assert!(sm.chat.shown);
        sm.sanitize();
        assert_eq!(sm.chat.share, 0.85);
    }
}
