use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{PanelNode, Schema, sanitize_panels};

/// 1.35, X4 Stream manager options: the activity-feed filters and the OBS status files.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
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
    /// V102-22 Your chat next to the cards. Its width is only read while `layout` is unset.
    pub chat: ManagerChat,
    /// V103-06 The panels as arranged; the ones missing wait in the closed-panels corner.
    /// `None` is the UI's default.
    #[ts(optional = nullable)]
    pub layout: Option<PanelNode>,
    /// V103-06 When on, panels can't be dragged or resized. On until the user unlocks it.
    pub locked: bool,
    /// V103-15 Which mod actions the "Mod actions" panel shows.
    pub mod_actions: ModActionFilters,
    /// V103-22 The chat panel brought back (`true`) or closed (`false`) by hand: it stays so
    /// until "Reset layout". `None` follows your channel's regular chat.
    #[ts(optional = nullable)]
    pub chat_kept: Option<bool>,
}

impl Default for StreamManager {
    fn default() -> Self {
        Self {
            activity: ActivityFilters::default(),
            obs: Obs::default(),
            alert_box: false,
            chat: ManagerChat::default(),
            layout: None,
            locked: true,
            mod_actions: ModActionFilters::default(),
            chat_kept: None,
        }
    }
}

/// V103-15 The kinds of mod action a "Mod actions" panel shows; all on until one is hidden.
/// Shared by the mod view (`modview.modActions`) and the stream manager.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct ModActionFilters {
    /// Bans and unbans.
    pub bans: bool,
    /// Timeouts and lifted timeouts.
    pub timeouts: bool,
    pub warnings: bool,
    /// Deleted messages and cleared chats.
    pub deletions: bool,
    /// Chat modes: slow, followers, subscribers, emote-only, unique chat, shield mode.
    pub modes: bool,
    /// Mods and VIPs added or removed.
    pub roles: bool,
    /// AutoMod decisions, blocked and allowed terms.
    pub automod: bool,
    /// Unban requests answered, raids and anything else.
    pub other: bool,
    /// V107-R30 Each line's date, and its time, shown; both on (the user's ask: every line
    /// alike, not the date only on older ones).
    pub date: bool,
    pub time: bool,
}

impl Default for ModActionFilters {
    fn default() -> Self {
        Self {
            bans: true,
            timeouts: true,
            warnings: true,
            deletions: true,
            modes: true,
            roles: true,
            automod: true,
            other: true,
            date: true,
            time: true,
        }
    }
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
        self.layout = sanitize_panels(self.layout.take());
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

    #[test]
    fn panels_start_locked_with_every_mod_action_shown() {
        // V103-06 Files from before the panel layout get it locked and unset.
        let sm: StreamManager = serde_json::from_str(r#"{"chat":{"shown":false}}"#).unwrap();
        assert!(sm.locked && sm.layout.is_none() && !sm.chat.shown);
        // V103-22 The chat follows the regular one until it's set by hand.
        assert_eq!(sm.chat_kept, None);
        let kept: StreamManager = serde_json::from_str(r#"{"chatKept":false}"#).unwrap();
        assert_eq!(kept.chat_kept, Some(false));
        let f: ModActionFilters = serde_json::from_str(r#"{"bans":false}"#).unwrap();
        assert!(!f.bans && f.timeouts && f.automod && f.other);
    }
}
