use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;
use super::emotes::{EmoteKey, EmoteSource, dedupe};

/// Twitch's longest timeout: 14 days.
const MAX_TIMEOUT_SECS: u32 = 1_209_600;
const MAX_TIMEOUT_PRESETS: usize = 8;
const MAX_NICKNAME_CHARS: usize = 25;
const MAX_NOTE_CHARS: usize = 500;
const MAX_HIDDEN_EMOTES: usize = 500;
/// E3 Percent of the normal size.
const EMOTE_SIZES: std::ops::RangeInclusive<u16> = 50..=300;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Chat {
    /// 2.3 Badges per provider, each its own toggle.
    pub badges: BadgeToggles,
    /// 2.3 Name paints per provider. BTTV's were dropped (D22).
    pub paints: PaintToggles,
    /// 2.3
    pub twitch_emotes: bool,
    /// 2.3 7TV, BTTV and FFZ emotes as one group.
    pub third_party_emotes: bool,
    /// 2.4
    pub unlisted_emotes: UnlistedEmotes,
    /// C11 Which provider wins when several have an emote by the same name.
    pub emote_priority: Vec<EmoteSource>,
    /// E3 Percent of the normal size.
    pub emote_size: u16,
    /// E2 Animated emotes and paints stand still while the window isn't focused.
    pub animate_only_focused: bool,
    /// E4 Shown as text everywhere.
    pub hidden_emotes: Vec<EmoteKey>,
    /// D21 Tell 7TV and BTTV which channel you're in, so others see your paint, badge
    /// and personal emotes.
    pub share_presence: bool,
    /// 2.5
    pub gifs: GifMode,
    /// 2.10
    pub timestamps: Timestamps,
    /// 2.11 Quick timeout durations in seconds, shortest first.
    pub timeout_presets: Vec<u32>,
    /// C1 Load recent messages when joining.
    pub history: bool,
    /// C14
    pub deleted: DeletedStyle,
    /// C15
    pub alternate_lines: bool,
    /// C15
    pub separators: bool,
    /// C20
    pub names: NameStyle,
    /// C5 Stop scrolling while the mouse is over chat.
    pub pause_on_hover: bool,
    /// C5 Stop scrolling while this key is held.
    pub pause_key: PauseKey,
    /// C5 Mark where you stopped reading when the window loses focus.
    pub last_read_line: bool,
    /// U6 Setting, off by default (D10).
    pub mention_sound: bool,
    /// U6 Setting, off by default (D10).
    pub mention_flash: bool,
    /// C3 Local nicknames and notes, by user id.
    pub users: BTreeMap<String, UserNote>,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            badges: BadgeToggles::default(),
            paints: PaintToggles::default(),
            twitch_emotes: true,
            third_party_emotes: true,
            unlisted_emotes: UnlistedEmotes::Blur,
            emote_priority: EmoteSource::THIRD_PARTY.to_vec(),
            emote_size: 100,
            animate_only_focused: false,
            hidden_emotes: Vec::new(),
            share_presence: true,
            gifs: GifMode::Show,
            timestamps: Timestamps::default(),
            timeout_presets: vec![60, 600, 3600, 86_400],
            history: true,
            deleted: DeletedStyle::Strike,
            alternate_lines: false,
            separators: false,
            names: NameStyle::Both,
            pause_on_hover: true,
            pause_key: PauseKey::Alt,
            last_read_line: true,
            mention_sound: false,
            mention_flash: false,
            users: BTreeMap::new(),
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
        // C11 Each third-party provider once; missing ones go last in the default order.
        let mut order: Vec<EmoteSource> = Vec::new();
        for s in self.emote_priority.iter().chain(EmoteSource::THIRD_PARTY.iter()) {
            if EmoteSource::THIRD_PARTY.contains(s) && !order.contains(s) {
                order.push(*s);
            }
        }
        self.emote_priority = order;
        self.emote_size = self.emote_size.clamp(*EMOTE_SIZES.start(), *EMOTE_SIZES.end());
        dedupe(&mut self.hidden_emotes, MAX_HIDDEN_EMOTES);
        self.users.retain(|id, u| {
            u.nickname = u.nickname.trim().chars().take(MAX_NICKNAME_CHARS).collect();
            u.note = u.note.trim().chars().take(MAX_NOTE_CHARS).collect();
            !id.is_empty() && !(u.nickname.is_empty() && u.note.is_empty())
        });
    }
}

/// C14 How deleted messages look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DeletedStyle {
    #[default]
    Strike,
    Faded,
    Hidden,
}

/// C20 Which name to show for users with a localized display name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NameStyle {
    Display,
    Login,
    /// Display name, plus the login when they differ.
    #[default]
    Both,
}

/// C5
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PauseKey {
    None,
    Ctrl,
    #[default]
    Alt,
    Shift,
}

/// C3
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct UserNote {
    pub nickname: String,
    pub note: String,
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
}

impl Default for PaintToggles {
    fn default() -> Self {
        Self { seventv: true }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_trims_notes_and_drops_empty_ones() {
        let mut c = Chat::default();
        c.users.insert(
            "1".into(),
            UserNote {
                nickname: format!("  {}  ", "n".repeat(40)),
                note: " hi ".into(),
            },
        );
        c.users.insert("2".into(), UserNote::default());
        c.timeout_presets = vec![600, 0, 60, 60, 2_000_000];
        c.sanitize();
        assert_eq!(c.users.len(), 1);
        assert_eq!(c.users["1"].nickname.len(), MAX_NICKNAME_CHARS);
        assert_eq!(c.users["1"].note, "hi");
        assert_eq!(c.timeout_presets, [60, 600]);
    }

    #[test]
    fn sanitize_completes_the_emote_priority() {
        let mut c = Chat {
            emote_priority: vec![EmoteSource::Ffz, EmoteSource::Twitch, EmoteSource::Ffz],
            emote_size: 1,
            ..Chat::default()
        };
        c.sanitize();
        assert_eq!(c.emote_priority, [EmoteSource::Ffz, EmoteSource::Seventv, EmoteSource::Bttv]);
        assert_eq!(c.emote_size, 50);
    }
}
