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

impl Schema for StreamManager {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_default_on() {
        let f = ActivityFilters::default();
        assert!(f.follows && f.subs && f.gifts && f.cheers && f.raids && f.redemptions && f.streaks);
        // Files saved before streaks existed get them on.
        let old: ActivityFilters = serde_json::from_str(r#"{"follows":false}"#).unwrap();
        assert!(!old.follows && old.streaks);
        assert!(!Obs::default().enabled);
    }
}
