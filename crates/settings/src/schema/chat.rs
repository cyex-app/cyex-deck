use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;
use super::emotes::{EmoteKey, EmoteSource, dedupe};

/// Twitch's longest timeout: 14 days.
const MAX_TIMEOUT_SECS: u32 = 1_209_600;
const MAX_TIMEOUT_PRESETS: usize = 8;
/// M10 Timeout buttons mod mode can show.
const MAX_MOD_TIMEOUTS: usize = 4;
/// V105-15 Entries in the button order, off timeouts included.
const MAX_MOD_ORDER: usize = 32;
const MAX_NICKNAME_CHARS: usize = 25;
const MAX_NOTE_CHARS: usize = 500;
const MAX_HIDDEN_EMOTES: usize = 500;
/// E3 Percent of the normal size.
const EMOTE_SIZES: std::ops::RangeInclusive<u16> = 50..=300;
/// C2 Percent alike that counts as a repeat.
const REPEAT_SIMILARITY: std::ops::RangeInclusive<u8> = 60..=100;
/// C19 Longest delay, in tenths of a second.
const MAX_DELAY_TENTHS: u16 = 600;
/// C17 A language code like `en` or `pt-BR`.
const MAX_LANGUAGE_CHARS: usize = 10;
/// V101-02 Messages a chat keeps (Chatterino allows up to 8.000).
pub const SCROLLBACK: std::ops::RangeInclusive<u32> = 100..=8000;
/// V101-03 History lines to ask for when a chat opens. The service keeps 800 today.
pub const HISTORY_LINES: std::ops::RangeInclusive<u32> = 50..=2000;
/// V101-15 Longest hover pause after the mouse stops, in tenths of a second.
const MAX_PAUSE_STILL_TENTHS: u16 = 600;

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
    /// V106-30 The user picked show or blur for unlisted emotes. Until then the emote card offers
    /// a quick "Show unlisted emotes".
    pub unlisted_chosen: bool,
    /// C11 Which provider wins when several have an emote by the same name.
    pub emote_priority: Vec<EmoteSource>,
    /// E3 Percent of the normal size.
    pub emote_size: u16,
    /// E2 Animated emotes and paints stand still while the window isn't focused.
    pub animate_only_focused: bool,
    /// E4 Shown as text everywhere.
    pub hidden_emotes: Vec<EmoteKey>,
    /// D21 V106-08 Send presence to 7TV and BTTV: which chat you're in, so others see your paint, badge and personal emotes.
    pub share_presence: bool,
    /// V106-08 Presence was turned on once for everyone.
    #[serde(default)]
    pub presence_on_once: bool,
    /// 2.5
    pub gifs: GifMode,
    /// 2.10
    pub timestamps: Timestamps,
    /// 2.11 Quick timeout durations in seconds, shortest first.
    pub timeout_presets: Vec<u32>,
    /// C1 Load recent messages when joining.
    pub history: bool,
    /// C37 Who joins and leaves, as lines. Twitch only tells in chats under 1.000 people.
    pub joins_leaves: bool,
    /// V101-03 How many, at most.
    pub history_lines: u32,
    /// V101-02 Messages each chat keeps; older ones drop off the top.
    pub scrollback: u32,
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
    /// V101-15 Once the mouse stops moving over chat, scrolling goes on after this long,
    /// in tenths of a second. 0: only when the mouse leaves.
    pub pause_still_tenths: u16,
    /// C5 Stop scrolling while this key is held.
    pub pause_key: PauseKey,
    /// C5 Mark where you stopped reading when the window loses focus.
    pub last_read_line: bool,
    /// V103-12 New messages slide in instead of popping up. Fast chats skip the slide.
    pub smooth_new_messages: bool,
    /// C2 One line with a count when someone repeats themselves. Setting, off.
    pub repeats: bool,
    /// C2 Percent alike that counts as a repeat.
    pub repeat_similarity: u8,
    /// C26 Show which app sent a message. Setting, off.
    pub show_client: bool,
    /// 1.52 Hover a link for a preview.
    pub link_previews: bool,
    /// 1.52
    pub link_thumbnails: LinkThumbs,
    /// C30 Link cards under messages too. Setting, off.
    pub link_cards: bool,
    /// C17 Language to translate into. Empty: the app's language.
    pub translate_to: String,
    /// C17 Chats (logins) whose messages are translated as they arrive.
    pub translate_chats: Vec<String>,
    /// V101-33 Chats (logins) in mod mode: delete, timeout and ban before each line.
    pub mod_mode: Vec<String>,
    /// M10 Which buttons mod mode shows.
    pub mod_buttons: ModButtons,
    /// C19 Hold chat back this long to match the stream, in tenths of a second. 0 is off.
    pub delay_tenths: u16,
    /// C18 Spell check in the message box. Setting, off.
    pub spell_check: bool,
    /// A send button inside the message box. Setting, on.
    pub send_button: bool,
    /// 1.42 Message effects and gigantified emotes as Twitch draws them.
    pub effects: bool,
    /// C3 Local nicknames and notes, by user id.
    pub users: BTreeMap<String, UserNote>,
    /// U7 How chat overlays look.
    pub overlay: Overlay,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            badges: BadgeToggles::default(),
            paints: PaintToggles::default(),
            twitch_emotes: true,
            third_party_emotes: true,
            unlisted_emotes: UnlistedEmotes::Blur,
            unlisted_chosen: false,
            emote_priority: EmoteSource::THIRD_PARTY.to_vec(),
            emote_size: 100,
            animate_only_focused: false,
            hidden_emotes: Vec::new(),
            share_presence: true,
            presence_on_once: true,
            gifs: GifMode::Show,
            timestamps: Timestamps::default(),
            timeout_presets: vec![60, 600, 3600, 86_400],
            history: true,
            joins_leaves: false,
            history_lines: 500,
            scrollback: 1000,
            deleted: DeletedStyle::Strike,
            alternate_lines: false,
            separators: false,
            names: NameStyle::Both,
            pause_on_hover: true,
            pause_still_tenths: 0,
            pause_key: PauseKey::Alt,
            last_read_line: true,
            smooth_new_messages: false,
            repeats: false,
            repeat_similarity: 80,
            show_client: false,
            link_previews: true,
            link_thumbnails: LinkThumbs::Medium,
            link_cards: false,
            translate_to: String::new(),
            translate_chats: Vec::new(),
            mod_mode: Vec::new(),
            mod_buttons: ModButtons::default(),
            delay_tenths: 0,
            spell_check: false,
            send_button: true,
            effects: true,
            users: BTreeMap::new(),
            overlay: Overlay::default(),
        }
    }
}

impl Schema for Chat {
    fn sanitize(&mut self) {
        // V106-30 Anyone on show or hide already chose. Never sets false.
        if self.unlisted_emotes != UnlistedEmotes::Blur {
            self.unlisted_chosen = true;
        }
        // V106-08 On for everyone once, including people who had it off.
        if !self.presence_on_once {
            self.share_presence = true;
            self.presence_on_once = true;
        }
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
        self.repeat_similarity = self.repeat_similarity.clamp(*REPEAT_SIMILARITY.start(), *REPEAT_SIMILARITY.end());
        self.delay_tenths = self.delay_tenths.min(MAX_DELAY_TENTHS);
        self.scrollback = self.scrollback.clamp(*SCROLLBACK.start(), *SCROLLBACK.end());
        self.history_lines = self.history_lines.clamp(*HISTORY_LINES.start(), *HISTORY_LINES.end());
        self.pause_still_tenths = self.pause_still_tenths.min(MAX_PAUSE_STILL_TENTHS);
        let lang = self.translate_to.trim();
        let valid = lang.len() <= MAX_LANGUAGE_CHARS && lang.chars().all(|c| c.is_ascii_alphabetic() || c == '-');
        self.translate_to = if valid { lang.to_owned() } else { String::new() };
        logins(&mut self.translate_chats);
        logins(&mut self.mod_mode);
        let t = &mut self.mod_buttons.timeouts;
        t.retain(|s| (1..=MAX_TIMEOUT_SECS).contains(s));
        t.sort_unstable();
        t.dedup();
        t.truncate(MAX_MOD_TIMEOUTS);
        // V105-15 Each button once; unpicked timeouts keep their place (they show as Add rows);
        // the rest last, in the default order.
        let b = &mut self.mod_buttons;
        let timeouts = b.timeouts.clone();
        let valid = |x: &ModButton| !matches!(x, ModButton::Timeout(s) if !(1..=MAX_TIMEOUT_SECS).contains(s));
        let mut kept: Vec<ModButton> = Vec::new();
        for x in b.order.iter().chain(default_order(&timeouts).iter()) {
            if valid(x) && !kept.contains(x) {
                kept.push(*x);
            }
        }
        kept.truncate(MAX_MOD_ORDER);
        b.order = kept;
        self.users.retain(|id, u| {
            u.nickname = u.nickname.trim().chars().take(MAX_NICKNAME_CHARS).collect();
            u.note = u.note.trim().chars().take(MAX_NOTE_CHARS).collect();
            !id.is_empty() && !(u.nickname.is_empty() && u.note.is_empty())
        });
        let o = &mut self.overlay;
        o.background = o.background.min(100);
        o.scale = o.scale.clamp(*OVERLAY_SCALES.start(), *OVERLAY_SCALES.end());
        o.fade_after = o.fade_after.min(MAX_OVERLAY_FADE);
    }
}

/// A list of channel logins: sorted, each once, no empty or overlong ones.
fn logins(list: &mut Vec<String>) {
    list.retain(|c| !c.is_empty() && c.len() <= 25);
    list.sort_unstable();
    list.dedup();
}

/// U7 Percent of the chat's text size in an overlay.
const OVERLAY_SCALES: std::ops::RangeInclusive<u16> = 50..=300;
/// U7 Longest wait before overlay lines fade, in seconds.
const MAX_OVERLAY_FADE: u16 = 600;

/// U7 A chat overlay: a see-through window over games or other apps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Overlay {
    /// How solid the background is, in percent. 0 is fully see-through.
    pub background: u8,
    /// Text size in percent of the chat's.
    pub scale: u16,
    /// Lines fade after this many seconds; 0 keeps them.
    pub fade_after: u16,
    /// A dark outline around letters, readable on any background.
    pub outline: bool,
    pub badges: bool,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            background: 25,
            scale: 100,
            fade_after: 0,
            outline: true,
            badges: true,
        }
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

/// V105-15 One of mod mode's buttons, for the order they show in. Each timeout length has
/// its own place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModButton {
    Delete,
    /// Seconds, one of `ModButtons::timeouts`.
    Timeout(u32),
    Warn,
    Ban,
    /// V107-R22b Lifts a ban or timeout, as /unban.
    Unban,
}

/// V105-15 The order with nothing dragged: delete, timeouts shortest first, warn, ban, unban.
fn default_order(timeouts: &[u32]) -> Vec<ModButton> {
    let mut out = vec![ModButton::Delete];
    out.extend(timeouts.iter().map(|&s| ModButton::Timeout(s)));
    out.extend([ModButton::Warn, ModButton::Ban, ModButton::Unban]);
    out
}

/// M10 Mod mode's buttons; V105-15 in the order the user dragged them into.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct ModButtons {
    pub delete: bool,
    /// Seconds, one button each.
    pub timeouts: Vec<u32>,
    /// Puts `/warn <login> ` in the chat box for the reason.
    pub warn: bool,
    pub ban: bool,
    /// V107-R22b Off by default; settings from before it load with it off.
    #[serde(default)]
    pub unban: bool,
    /// V105-15 Left to right. Each button once; timeouts only while picked. Settings from
    /// before this field start empty, so `sanitize` fills in the default order.
    #[serde(default)]
    pub order: Vec<ModButton>,
}

impl Default for ModButtons {
    fn default() -> Self {
        Self {
            delete: true,
            timeouts: vec![60],
            warn: false,
            ban: true,
            unban: false,
            order: default_order(&[60]),
        }
    }
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
    /// V107-R6 A thin dark outline under painted names in light themes, so pale paints stay
    /// readable. The paint's own colors stay as 7TV draws them.
    pub outline_light: bool,
}

impl Default for PaintToggles {
    fn default() -> Self {
        Self {
            seventv: true,
            outline_light: true,
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

/// 1.52 Preview picture size: none, or 100, 200, 300 px wide (Chatterino's sizes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LinkThumbs {
    Off,
    Small,
    #[default]
    Medium,
    Large,
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
    fn paint_outline_defaults_on_for_old_files() {
        // V107-R6 A file saved before the outline setting keeps paints and gets the outline.
        let p: PaintToggles = serde_json::from_str(r#"{"seventv":false}"#).unwrap();
        assert!(!p.seventv);
        assert!(p.outline_light);
        let off: PaintToggles = serde_json::from_str(r#"{"outlineLight":false}"#).unwrap();
        assert!(off.seventv && !off.outline_light);
    }

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

    #[test]
    fn scrollback_and_history_stay_in_range() {
        let d = Chat::default();
        assert_eq!((d.scrollback, d.history_lines, d.pause_still_tenths), (1000, 500, 0));
        let mut c = Chat {
            scrollback: 9_000_000,
            history_lines: 0,
            pause_still_tenths: 10_000,
            ..Chat::default()
        };
        c.sanitize();
        assert_eq!((c.scrollback, c.history_lines, c.pause_still_tenths), (8000, 50, 600));
        let old: Chat = serde_json::from_str(r#"{"history":false}"#).unwrap();
        assert!(!old.history);
        assert_eq!(old.scrollback, 1000);
    }

    #[test]
    fn mod_mode_keeps_each_chat_once() {
        let mut c = Chat {
            mod_mode: vec!["b".into(), String::new(), "a".into(), "b".into(), "x".repeat(26)],
            ..Chat::default()
        };
        c.sanitize();
        assert_eq!(c.mod_mode, ["a", "b"]);
        assert!(Chat::default().mod_mode.is_empty());
    }

    #[test]
    fn mod_buttons_keep_a_few_real_timeouts() {
        let mut c = Chat::default();
        c.mod_buttons.timeouts = vec![600, 0, 60, 60, 2_000_000, 5, 30, 90];
        c.sanitize();
        assert_eq!(c.mod_buttons.timeouts, [5, 30, 60, 90]);
        assert_eq!(Chat::default().mod_buttons.timeouts, [60]);
    }

    #[test]
    fn mod_button_order_keeps_each_once() {
        use ModButton::*;
        let mut c = Chat::default();
        c.mod_buttons.timeouts = vec![60, 600];
        c.mod_buttons.order = vec![Ban, Timeout(600), Ban, Timeout(999)];
        c.sanitize();
        assert_eq!(c.mod_buttons.order, [Ban, Timeout(600), Timeout(999), Delete, Timeout(60), Warn, Unban]);
        c.mod_buttons.order = vec![Ban, Timeout(0), Timeout(2_000_000)];
        c.sanitize();
        assert_eq!(c.mod_buttons.order, [Ban, Delete, Timeout(60), Timeout(600), Warn, Unban]);
    }

    #[test]
    fn mod_button_order_keeps_unpicked_timeouts() {
        use ModButton::*;
        let mut c = Chat::default();
        c.mod_buttons.timeouts = vec![60];
        c.mod_buttons.order = vec![Timeout(600), Delete, Timeout(60), Warn, Ban];
        c.sanitize();
        assert_eq!(c.mod_buttons.order, [Timeout(600), Delete, Timeout(60), Warn, Ban, Unban]);
    }

    #[test]
    fn mod_button_order_defaults() {
        use ModButton::*;
        let mut c = Chat::default();
        c.mod_buttons.timeouts = vec![30, 60];
        c.mod_buttons.order = vec![];
        c.sanitize();
        assert_eq!(c.mod_buttons.order, [Delete, Timeout(30), Timeout(60), Warn, Ban, Unban]);
        assert_eq!(Chat::default().mod_buttons.order, [Delete, Timeout(60), Warn, Ban, Unban]);
    }

    #[test]
    fn send_button_is_on_by_default_and_when_missing() {
        assert!(Chat::default().send_button);
        let c: Chat = serde_json::from_str(r#"{"spellCheck":true}"#).unwrap();
        assert!(c.send_button);
        let c: Chat = serde_json::from_str(r#"{"sendButton":false}"#).unwrap();
        assert!(!c.send_button);
    }

    #[test]
    fn old_settings_get_the_default_order() {
        use ModButton::*;
        let mut c: Chat =
            serde_json::from_str(r#"{"modButtons":{"delete":true,"timeouts":[60,600],"warn":false,"ban":true}}"#).unwrap();
        c.sanitize();
        assert_eq!(c.mod_buttons.order, [Delete, Timeout(60), Timeout(600), Warn, Ban, Unban]);
        assert!(!c.mod_buttons.unban, "V107-R22b old files load with Unban off");
    }

    #[test]
    fn v107_r22b_unban_is_off_by_default_and_kept_when_on() {
        use ModButton::*;
        assert!(!Chat::default().mod_buttons.unban);
        let mut c: Chat = serde_json::from_str(
            r#"{"modButtons":{"delete":true,"timeouts":[60],"warn":false,"ban":true,"unban":true,"order":["unban","ban"]}}"#,
        )
        .unwrap();
        c.sanitize();
        assert!(c.mod_buttons.unban);
        assert_eq!(c.mod_buttons.order, [Unban, Ban, Delete, Timeout(60), Warn]);
    }

    #[test]
    fn presence_turns_on_once() {
        let mut c: Chat = serde_json::from_str(r#"{"sharePresence":false}"#).unwrap();
        c.sanitize();
        assert!(c.share_presence && c.presence_on_once);
        c.share_presence = false;
        c.sanitize();
        assert!(!c.share_presence);
    }

    #[test]
    fn unlisted_chosen_follows_the_mode() {
        let read = |j: &str| {
            let mut c: Chat = serde_json::from_str(j).unwrap();
            c.sanitize();
            c.unlisted_chosen
        };
        assert!(!read("{}"));
        assert!(read(r#"{"unlistedEmotes":"show"}"#));
        assert!(read(r#"{"unlistedEmotes":"hide"}"#));
        assert!(read(r#"{"unlistedEmotes":"blur","unlistedChosen":true}"#));
        assert!(!read(r#"{"unlistedEmotes":"blur"}"#));
    }

    #[test]
    fn new_settings_need_no_presence_migration() {
        assert!(Chat::default().presence_on_once);
    }

    #[test]
    fn old_file_without_flag_reads_false() {
        assert!(!serde_json::from_str::<Chat>("{}").unwrap().presence_on_once);
    }
}
