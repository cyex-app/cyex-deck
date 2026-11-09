use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;
use super::emotes::{EmoteKey, EmoteSource, dedupe};
use super::highlights::clean_color;
use super::name_clicks::NameClicks;

/// Twitch's longest timeout: 14 days.
const MAX_TIMEOUT_SECS: u32 = 1_209_600;
const MAX_TIMEOUT_PRESETS: usize = 8;
/// M10 Timeout buttons mod mode can show.
const MAX_MOD_TIMEOUTS: usize = 4;
/// V110-M23 Slow mode lengths in seconds (Twitch: 3 to 120), follow ages in minutes (0 is
/// "any follower" and always there; up to 3 months), ad lengths in seconds (Twitch: 30 to
/// 180 in steps of 30). Each list holds at most this many.
const SLOW_SECS: std::ops::RangeInclusive<u32> = 3..=120;
const FOLLOW_MINUTES: std::ops::RangeInclusive<u32> = 1..=129_600;
const AD_SECS: [u32; 6] = [30, 60, 90, 120, 150, 180];
const MAX_MODE_PRESETS: usize = 10;
/// V105-15 Entries in the button order, off timeouts included.
const MAX_MOD_ORDER: usize = 32;
const MAX_NICKNAME_CHARS: usize = 25;
const MAX_NOTE_CHARS: usize = 500;
const MAX_HIDDEN_EMOTES: usize = 500;
/// V108-N20 Saved phrases and the longest one (Twitch's limit).
const MAX_QUICK_REPLIES: usize = 20;
const MAX_QUICK_REPLY_CHARS: usize = 500;
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
    /// V110-E13 7TV emotes. Off: not fetched, not drawn, not offered, not in the order.
    pub seventv_emotes: bool,
    /// V110-E13 BTTV emotes, as `seventv_emotes`.
    pub bttv_emotes: bool,
    /// V110-E13 FFZ emotes, as `seventv_emotes`.
    pub ffz_emotes: bool,
    /// V110-E13 The one switch from before for all three: read once, then the three take over.
    /// Never written back.
    #[serde(rename = "thirdPartyEmotes", skip_serializing)]
    #[ts(skip)]
    pub old_third_party_emotes: Option<bool>,
    /// 2.4
    pub unlisted_emotes: UnlistedEmotes,
    /// V106-30 The user picked show or blur for unlisted emotes. Until then the emote card offers
    /// a quick "Show unlisted emotes".
    pub unlisted_chosen: bool,
    /// C11 Which provider wins when several have an emote by the same name. V110-U71 Twitch is in
    /// the list too (files from before have it added first), and Tab completion uses it as well.
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
    /// V108-C31 A line in chat when a channel's 7TV, BTTV or FFZ emotes change.
    pub emote_changes: bool,
    /// 2.5
    pub gifs: GifMode,
    /// 2.10
    pub timestamps: Timestamps,
    /// 2.11 Quick timeout durations in seconds, shortest first.
    pub timeout_presets: Vec<u32>,
    /// V110-M23 Slow mode lengths in seconds, shortest first.
    pub slow_presets: Vec<u32>,
    /// V110-M23 Followers-only ages in minutes, shortest first. "Any follower" is always offered.
    pub follow_presets: Vec<u32>,
    /// V110-M23 Ad lengths in seconds, shortest first.
    pub ad_presets: Vec<u32>,
    /// V110-M23 The ad length picked when the stream manager opens.
    pub ad_length: u32,
    /// V110-M28 While banned in a chat, keep reading it anonymously.
    pub read_while_banned: bool,
    /// C1 Load recent messages when joining.
    pub history: bool,
    /// C37 V108-C02 Who joins the chat, as lines. Only in chats under 1.000 viewers.
    pub joins: bool,
    /// C37 V108-C02 Who leaves the chat, as lines. Only in chats under 1.000 viewers.
    pub parts: bool,
    /// V108-C02 The one joins-and-leaves toggle from before: read once, then `joins` and
    /// `parts` take over. Never written back.
    #[serde(rename = "joinsLeaves", skip_serializing)]
    #[ts(skip)]
    pub old_joins_leaves: bool,
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
    /// V110-C55 Ticks on the scrollbar at highlights, mentions and event lines. Click one to go there.
    pub scroll_marks: bool,
    /// V110-M20 What clicking a name does (click, middle, Ctrl, Shift).
    pub name_clicks: NameClicks,
    /// V103-12 New messages slide in instead of popping up. Fast chats skip the slide.
    pub smooth_new_messages: bool,
    /// C2 One line with a count when someone repeats themselves. Setting, off.
    pub repeats: bool,
    /// C2 Percent alike that counts as a repeat.
    pub repeat_similarity: u8,
    /// C26 Show which app sent a message. Setting, off.
    pub show_client: bool,
    /// V108-N10 Messages per second in the chat header. Setting, off.
    pub rate_meter: bool,
    /// V108-N04 The hype train bar over the chat while a train runs.
    pub hype_train_bar: bool,
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
    /// V108-MC Tab at the start of a message adds a comma: "@name, ". Setting, off.
    pub mention_comma: bool,
    /// A send button inside the message box. Setting, on.
    pub send_button: bool,
    /// V108-N09 `:` suggestions as a list above the box, or as a row.
    pub suggest_style: SuggestStyle,
    /// V108-N20 Phrases one click from the message box; a click puts one in the box.
    pub quick_replies: Vec<String>,
    /// V107-Bn Cheers from this many Bits up ask first. 1: every cheer asks.
    pub cheer_confirm_bits: u32,
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
            seventv_emotes: true,
            bttv_emotes: true,
            ffz_emotes: true,
            old_third_party_emotes: None,
            unlisted_emotes: UnlistedEmotes::Blur,
            unlisted_chosen: false,
            emote_priority: EmoteSource::RANKED.to_vec(),
            emote_size: 100,
            animate_only_focused: false,
            hidden_emotes: Vec::new(),
            share_presence: true,
            presence_on_once: true,
            emote_changes: true,
            gifs: GifMode::Show,
            timestamps: Timestamps::default(),
            timeout_presets: vec![60, 600, 3600, 86_400],
            slow_presets: vec![3, 5, 10, 20, 30, 60, 120],
            follow_presets: vec![10, 30, 60, 1440, 10_080, 43_200, 129_600],
            ad_presets: AD_SECS[..5].to_vec(),
            ad_length: 60,
            read_while_banned: true,
            history: true,
            joins: false,
            parts: false,
            old_joins_leaves: false,
            history_lines: 500,
            scrollback: 1000,
            deleted: DeletedStyle::Strike,
            alternate_lines: false,
            separators: false,
            names: NameStyle::DisplayLogin,
            pause_on_hover: true,
            pause_still_tenths: 0,
            pause_key: PauseKey::Alt,
            last_read_line: true,
            scroll_marks: true,
            name_clicks: NameClicks::default(),
            smooth_new_messages: false,
            repeats: false,
            repeat_similarity: 80,
            show_client: false,
            rate_meter: false,
            hype_train_bar: true,
            link_previews: true,
            link_thumbnails: LinkThumbs::Medium,
            link_cards: false,
            translate_to: String::new(),
            translate_chats: Vec::new(),
            mod_mode: Vec::new(),
            mod_buttons: ModButtons::default(),
            delay_tenths: 0,
            spell_check: false,
            mention_comma: false,
            send_button: true,
            suggest_style: SuggestStyle::List,
            quick_replies: Vec::new(),
            cheer_confirm_bits: 1000,
            effects: true,
            users: BTreeMap::new(),
            overlay: Overlay::default(),
        }
    }
}

impl Schema for Chat {
    fn sanitize(&mut self) {
        self.cheer_confirm_bits = self.cheer_confirm_bits.max(1);
        self.name_clicks.sanitize();
        // V106-30 Anyone on show or hide already chose. Never sets false.
        if self.unlisted_emotes != UnlistedEmotes::Blur {
            self.unlisted_chosen = true;
        }
        // V106-08 On for everyone once, including people who had it off.
        if !self.presence_on_once {
            self.share_presence = true;
            self.presence_on_once = true;
        }
        // V108-C02 The old toggle on turns both new ones on, once.
        if std::mem::take(&mut self.old_joins_leaves) {
            self.joins = true;
            self.parts = true;
        }
        // V110-E13 The old switch for all three sets each of them, once.
        if let Some(on) = self.old_third_party_emotes.take() {
            self.seventv_emotes = on;
            self.bttv_emotes = on;
            self.ffz_emotes = on;
        }
        let presets = &mut self.timeout_presets;
        presets.retain(|s| (1..=MAX_TIMEOUT_SECS).contains(s));
        presets.sort_unstable();
        presets.dedup();
        presets.truncate(MAX_TIMEOUT_PRESETS);
        // V110-M23 Each list: in range, sorted, once, never empty (empty gets the defaults).
        let fix = |list: &mut Vec<u32>, ok: &dyn Fn(u32) -> bool, default: Vec<u32>| {
            list.retain(|s| ok(*s));
            list.sort_unstable();
            list.dedup();
            list.truncate(MAX_MODE_PRESETS);
            if list.is_empty() {
                *list = default;
            }
        };
        let d = Self::default();
        fix(&mut self.slow_presets, &|s| SLOW_SECS.contains(&s), d.slow_presets);
        fix(&mut self.follow_presets, &|s| FOLLOW_MINUTES.contains(&s), d.follow_presets);
        fix(&mut self.ad_presets, &|s| AD_SECS.contains(&s), d.ad_presets);
        // The default is one of the lengths on offer.
        if !self.ad_presets.contains(&self.ad_length) {
            self.ad_length = if self.ad_presets.contains(&60) { 60 } else { self.ad_presets[0] };
        }
        // C11 Each provider once; V110-U71 an old list without Twitch gets it first (as it
        // always ranked), other missing ones go last in the default order.
        let mut order: Vec<EmoteSource> = Vec::new();
        let had_twitch = self.emote_priority.contains(&EmoteSource::Twitch);
        if !had_twitch {
            order.push(EmoteSource::Twitch);
        }
        for s in self.emote_priority.iter().chain(EmoteSource::RANKED.iter()) {
            if EmoteSource::RANKED.contains(s) && !order.contains(s) {
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
            clean_color(&mut u.color);
            !id.is_empty() && !(u.nickname.is_empty() && u.note.is_empty() && u.color.is_empty())
        });
        // V108-N20 Trimmed, no empty or repeated phrase, at most 20.
        let mut seen = std::collections::HashSet::new();
        self.quick_replies = std::mem::take(&mut self.quick_replies)
            .into_iter()
            .map(|r| r.trim().chars().take(MAX_QUICK_REPLY_CHARS).collect::<String>())
            .filter(|r| !r.is_empty() && seen.insert(r.clone()))
            .take(MAX_QUICK_REPLIES)
            .collect();
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

/// C20 V108-C20 Which name comes first in the chat box, and whether the other shows in
/// brackets. The bracket part only shows when the two differ beyond letter case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NameStyle {
    /// Display name only.
    Display,
    /// Display name, then the login in brackets. Was `both`, which still loads.
    #[default]
    #[serde(alias = "both")]
    DisplayLogin,
    /// Login only, for everyone. Was `login`, which still loads.
    #[serde(alias = "login")]
    Username,
    /// Login, then the display name in brackets.
    UsernameDisplay,
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
    /// V108-N08 Own name colour for this person (`#rrggbb`), empty for theirs.
    pub color: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeToggles {
    pub twitch: bool,
    pub seventv: bool,
    pub bttv: bool,
    pub ffz: bool,
    /// V109-B02 cyex badges (founder, supporter, supporter+). Off: their list isn't fetched.
    pub cyex: bool,
}

impl Default for BadgeToggles {
    fn default() -> Self {
        Self {
            twitch: true,
            seventv: true,
            bttv: true,
            ffz: true,
            cyex: true,
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

/// V108-N09 How `:` suggestions are laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SuggestStyle {
    #[default]
    List,
    Row,
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
#[serde(from = "TimestampsFile", rename_all = "camelCase")]
#[ts(export)]
pub struct Timestamps {
    pub show: bool,
    pub seconds: bool,
    /// V110-U53 24-hour by default (3.2). Files from before read the old `hour12` bool.
    pub clock: ClockFormat,
}

/// What a settings file holds for [`Timestamps`]: `clock`, and `hour12` from before it. A file
/// with both loads (V110-F17), `clock` winning.
#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct TimestampsFile {
    show: bool,
    seconds: bool,
    clock: Option<ClockFormat>,
    hour12: Option<ClockFormat>,
}

impl Default for TimestampsFile {
    fn default() -> Self {
        let d = Timestamps::default();
        Self { show: d.show, seconds: d.seconds, clock: None, hour12: None }
    }
}

impl From<TimestampsFile> for Timestamps {
    fn from(f: TimestampsFile) -> Self {
        Self {
            show: f.show,
            seconds: f.seconds,
            clock: f.clock.or(f.hour12).unwrap_or_default(),
        }
    }
}

/// V110-U53 The clock timestamps use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClockFormat {
    #[default]
    H24,
    H12,
}

impl<'de> Deserialize<'de> for ClockFormat {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Old(bool),
            Text(String),
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Old(true) => Self::H12,
            Raw::Text(t) if t == "h12" => Self::H12,
            Raw::Text(t) if t == "h24" => Self::H24,
            Raw::Text(t) => {
                eprintln!("settings: unknown clock {t:?}, using 24-hour");
                Self::H24
            }
            Raw::Old(false) => Self::H24,
        })
    }
}

impl Default for Timestamps {
    fn default() -> Self {
        Self {
            show: true,
            seconds: false,
            clock: ClockFormat::H24,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v110_u53_clock_reads_the_old_bool_and_the_new_enum() {
        let old: Chat = serde_json::from_str(r#"{"timestamps":{"hour12":true}}"#).unwrap();
        assert_eq!(old.timestamps.clock, ClockFormat::H12);
        // V110-F17 Both keys in one file load, `clock` winning; unknown text falls back to 24-hour.
        let both: Chat =
            serde_json::from_str(r#"{"timestamps":{"hour12":true,"clock":"h24","show":false}}"#).unwrap();
        assert_eq!((both.timestamps.clock, both.timestamps.show), (ClockFormat::H24, false));
        let both: Chat = serde_json::from_str(r#"{"timestamps":{"hour12":false,"clock":"h12"}}"#).unwrap();
        assert_eq!(both.timestamps.clock, ClockFormat::H12);
        let odd: Chat = serde_json::from_str(r#"{"timestamps":{"clock":"h99"}}"#).unwrap();
        assert_eq!(odd.timestamps.clock, ClockFormat::H24);
        let old: Chat = serde_json::from_str(r#"{"timestamps":{"hour12":false}}"#).unwrap();
        assert_eq!(old.timestamps.clock, ClockFormat::H24);
        let new: Chat = serde_json::from_str(r#"{"timestamps":{"clock":"h12"}}"#).unwrap();
        assert_eq!(new.timestamps.clock, ClockFormat::H12);
        let v = serde_json::to_value(&new).unwrap();
        assert_eq!(v["timestamps"]["clock"], "h12");
        assert_eq!(v["timestamps"].get("hour12"), None);
        assert_eq!(Chat::default().timestamps.clock, ClockFormat::H24);
    }

    #[test]
    fn v110_u43_old_raid_join_key_is_ignored() {
        let c: Chat = serde_json::from_str(r#"{"raidJoin":true,"rateMeter":true}"#).unwrap();
        assert!(c.rate_meter, "the rest of the file still loads");
        assert_eq!(serde_json::to_value(&c).unwrap().get("raidJoin"), None, "and the key is gone on the next save");
    }

    #[test]
    fn v109_b02_cyex_badges_start_on_for_old_files() {
        let b: BadgeToggles = serde_json::from_str(r#"{"seventv":false}"#).unwrap();
        assert!(b.cyex && !b.seventv);
        let off: BadgeToggles = serde_json::from_str(r#"{"cyex":false}"#).unwrap();
        assert!(!off.cyex && off.twitch);
    }

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
    fn quick_replies_are_trimmed_unique_and_capped() {
        let mut c = Chat {
            quick_replies: vec!["  gg  ".into(), "".into(), "gg".into(), "x".repeat(600)],
            ..Default::default()
        };
        c.quick_replies.extend((0..30).map(|i| format!("r{i}")));
        c.sanitize();
        assert_eq!(c.quick_replies.len(), MAX_QUICK_REPLIES);
        assert_eq!(c.quick_replies[0], "gg");
        assert_eq!(c.quick_replies[1].chars().count(), MAX_QUICK_REPLY_CHARS);
        assert_eq!(c.suggest_style, SuggestStyle::List);
    }

    #[test]
    fn sanitize_trims_notes_and_drops_empty_ones() {
        let mut c = Chat::default();
        c.users.insert(
            "1".into(),
            UserNote {
                nickname: format!("  {}  ", "n".repeat(40)),
                note: " hi ".into(),
                ..Default::default()
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
    fn sanitize_mode_presets() {
        let mut c = Chat {
            slow_presets: vec![30, 1, 30, 500, 5],
            follow_presets: vec![],
            ad_presets: vec![45, 90, 30],
            ad_length: 45,
            ..Chat::default()
        };
        c.sanitize();
        assert_eq!(c.slow_presets, [5, 30]);
        assert_eq!(c.follow_presets, Chat::default().follow_presets);
        assert_eq!(c.ad_presets, [30, 90]);
        assert_eq!(c.ad_length, 30);
        assert!(Chat::default().read_while_banned);
    }

    #[test]
    fn sanitize_completes_the_emote_priority() {
        let mut c = Chat {
            emote_priority: vec![EmoteSource::Ffz, EmoteSource::Twitch, EmoteSource::Ffz],
            emote_size: 1,
            ..Chat::default()
        };
        c.sanitize();
        assert_eq!(
            c.emote_priority,
            [EmoteSource::Ffz, EmoteSource::Twitch, EmoteSource::Seventv, EmoteSource::Bttv]
        );
        assert_eq!(c.emote_size, 50);
    }

    #[test]
    fn v110_u71_an_old_priority_without_twitch_keeps_it_first() {
        let mut c = Chat {
            emote_priority: vec![EmoteSource::Bttv, EmoteSource::Seventv, EmoteSource::Ffz],
            ..Chat::default()
        };
        c.sanitize();
        assert_eq!(
            c.emote_priority,
            [EmoteSource::Twitch, EmoteSource::Bttv, EmoteSource::Seventv, EmoteSource::Ffz]
        );
        assert_eq!(Chat::default().emote_priority, EmoteSource::RANKED);
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
    fn v107_bn_cheer_confirm_defaults_to_1000_and_never_0() {
        let c: Chat = serde_json::from_str(r#"{"spellCheck":true}"#).unwrap();
        assert_eq!(c.cheer_confirm_bits, 1000);
        let mut c: Chat = serde_json::from_str(r#"{"cheerConfirmBits":0}"#).unwrap();
        c.sanitize();
        assert_eq!(c.cheer_confirm_bits, 1);
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
    fn v108_02_old_joins_leaves_turns_both_on() {
        let read = |j: &str| {
            let mut c: Chat = serde_json::from_str(j).unwrap();
            c.sanitize();
            c
        };
        let c = read(r#"{"joinsLeaves":true}"#);
        assert!(c.joins && c.parts);
        // Written back without the old field, so turning one off sticks.
        let saved = serde_json::to_value(&c).unwrap();
        assert!(saved.get("joinsLeaves").is_none());
        assert_eq!((saved["joins"].as_bool(), saved["parts"].as_bool()), (Some(true), Some(true)));
        let mut again: Chat = serde_json::from_value(saved).unwrap();
        again.parts = false;
        again.sanitize();
        assert!(again.joins && !again.parts);
        let off = read(r#"{"joinsLeaves":false}"#);
        assert!(!off.joins && !off.parts);
        let d = Chat::default();
        assert!(!d.joins && !d.parts && !d.old_joins_leaves);
        let new = read(r#"{"joins":true}"#);
        assert!(new.joins && !new.parts);
    }

    #[test]
    fn v110_e13_old_third_party_switch_sets_all_three() {
        let read = |j: &str| {
            let mut c: Chat = serde_json::from_str(j).unwrap();
            c.sanitize();
            c
        };
        let off = read(r#"{"thirdPartyEmotes":false,"someFutureKey":1}"#);
        assert!(!off.seventv_emotes && !off.bttv_emotes && !off.ffz_emotes);
        let on = read(r#"{"thirdPartyEmotes":true}"#);
        assert!(on.seventv_emotes && on.bttv_emotes && on.ffz_emotes);
        // Written back without the old key, so one turned on again sticks.
        let mut saved = serde_json::to_value(&off).unwrap();
        assert!(saved.get("thirdPartyEmotes").is_none());
        saved["bttvEmotes"] = true.into();
        let again = read(&saved.to_string());
        assert!(!again.seventv_emotes && again.bttv_emotes && !again.ffz_emotes);
        let d = read("{}");
        assert!(d.seventv_emotes && d.bttv_emotes && d.ffz_emotes);
    }

    #[test]
    fn new_settings_need_no_presence_migration() {
        assert!(Chat::default().presence_on_once);
    }

    #[test]
    fn old_file_without_flag_reads_false() {
        assert!(!serde_json::from_str::<Chat>("{}").unwrap().presence_on_once);
    }

    #[test]
    fn v108_c20_old_name_styles_still_load() {
        let read = |v: &str| {
            serde_json::from_str::<Chat>(&format!(r#"{{"names":"{v}"}}"#)).unwrap().names
        };
        assert_eq!(read("display"), NameStyle::Display);
        assert_eq!(read("login"), NameStyle::Username);
        assert_eq!(read("both"), NameStyle::DisplayLogin);
        assert_eq!(read("usernameDisplay"), NameStyle::UsernameDisplay);
        assert_eq!(read("displayLogin"), NameStyle::DisplayLogin);
        assert_eq!(serde_json::to_string(&NameStyle::Username).unwrap(), r#""username""#);
        assert_eq!(Chat::default().names, NameStyle::DisplayLogin);
    }
}
