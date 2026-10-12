use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// 2.2, 1.26 The app's own alerts: followed channels going live, whispers, and Twitch's
/// bell. Twitch's email and push switches live on Twitch, not here.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Notifications {
    pub live: LiveAlerts,
    pub whispers: WhisperAlerts,
    pub bell: BellAlerts,
}

/// 2.2 Alerts when followed channels go live. Which channels: Twitch's bell for each one,
/// unless the user picked on or off here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct LiveAlerts {
    pub enabled: bool,
    /// Channel id → on or off, over Twitch's bell.
    pub channels: BTreeMap<String, bool>,
    /// A Windows pop-up.
    pub toast: bool,
    pub sound: bool,
    /// Skip channels already live when the app starts.
    pub skip_at_start: bool,
}

impl Default for LiveAlerts {
    fn default() -> Self {
        Self {
            enabled: true,
            channels: BTreeMap::new(),
            toast: true,
            sound: false,
            skip_at_start: true,
        }
    }
}

impl LiveAlerts {
    /// Whether a live channel alerts, given its Twitch bell.
    pub fn wants(&self, channel_id: &str, bell: bool) -> bool {
        self.enabled && self.channels.get(channel_id).copied().unwrap_or(bell)
    }
}

/// 1.26 What a new whisper does while the app isn't in front.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperAlerts {
    pub toast: bool,
    pub sound: bool,
    /// Flash the taskbar button.
    pub flash: bool,
    /// Also show whispers as lines in the chats on screen, like Chatterino.
    pub inline: bool,
}

impl Default for WhisperAlerts {
    fn default() -> Self {
        Self {
            toast: true,
            sound: true,
            flash: true,
            inline: true,
        }
    }
}

/// 1.26 Twitch's bell.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct BellAlerts {
    /// A Windows pop-up when Twitch marks a bell item as one.
    pub toast: bool,
}

impl Default for BellAlerts {
    fn default() -> Self {
        Self { toast: true }
    }
}

impl Schema for Notifications {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_choice_beats_the_bell() {
        let mut l = LiveAlerts::default();
        assert!(l.wants("1", true) && !l.wants("1", false));
        l.channels.insert("1".into(), false);
        l.channels.insert("2".into(), true);
        assert!(!l.wants("1", true) && l.wants("2", false));
        l.enabled = false;
        assert!(!l.wants("2", true));
    }

    #[test]
    fn old_empty_file_reads() {
        let n: Notifications = serde_json::from_str("{}").unwrap();
        assert_eq!(n, Notifications::default());
        assert!(n.whispers.inline && n.bell.toast && n.live.skip_at_start);
    }
}
