//! The rule types of the highlights partition.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub(super) const MAX_PATTERN_CHARS: usize = 500;
pub(super) const MAX_ROUTE_CHARS: usize = 40;
const MAX_PATH_CHARS: usize = 1000;
/// Sound files the window can play.
pub const SOUND_TYPES: [&str; 6] = ["mp3", "wav", "ogg", "opus", "flac", "m4a"];

/// `#rrggbb` or `#rrggbbaa`, lowercase; anything else becomes empty (the default color).
pub(crate) fn clean_color(color: &mut String) {
    let hex = color.strip_prefix('#').unwrap_or("");
    if !(matches!(hex.len(), 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())) {
        color.clear();
    }
    color.make_ascii_lowercase();
}

/// A playable sound file path, or empty.
pub(crate) fn clean_sound_file(path: &mut String) {
    *path = path.trim().to_owned();
    if !is_sound_file(path) || path.chars().count() > MAX_PATH_CHARS {
        path.clear();
    }
}

/// A file name the sound option takes: one of [`SOUND_TYPES`].
pub fn is_sound_file(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| SOUND_TYPES.iter().any(|t| t.eq_ignore_ascii_case(ext)))
}

/// Which regex syntax a pattern is written in. One engine runs them all (V104-01).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum RegexFlavor {
    #[default]
    Rust,
    JavaScript,
    Python,
}

/// What a matching message does (2.6), shared by every rule.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Alert {
    /// `#rrggbbaa`. Empty uses the highlight's own color.
    pub color: String,
    pub sound: bool,
    pub flash: bool,
    /// V105-17 Also copy the message into the Mentions tab.
    pub show: bool,
    /// V106-33 A Windows pop-up.
    pub notify: bool,
    /// V105-17 A sound file played instead of the chime. Empty: the chime.
    pub sound_file: String,
    /// F4 Also copy the message into the feed tab with this name. Empty: nowhere.
    pub route: String,
}

impl Alert {
    pub(super) fn sanitize(&mut self) {
        clean_color(&mut self.color);
        clean_sound_file(&mut self.sound_file);
        self.route = self.route.trim().chars().take(MAX_ROUTE_CHARS).collect();
    }
}

/// A built-in highlight: on or off, with its alert.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct BuiltIn {
    pub enabled: bool,
    #[serde(flatten)]
    #[ts(flatten)]
    pub alert: Alert,
}

impl BuiltIn {
    pub(super) const fn on() -> Self {
        Self {
            enabled: true,
            alert: Alert {
                color: String::new(),
                sound: false,
                flash: false,
                show: false,
                notify: false,
                sound_file: String::new(),
                route: String::new(),
            },
        }
    }

    pub(super) const fn off() -> Self {
        let mut b = Self::on();
        b.enabled = false;
        b
    }

    /// V106-33 Your name and replies: on, always in the Mentions tab.
    pub(super) const fn you(sound: bool) -> Self {
        let mut b = Self::on();
        b.alert.show = true;
        b.alert.sound = sound;
        b
    }
}

impl Default for BuiltIn {
    fn default() -> Self {
        Self::on()
    }
}

/// Words or a regex in the message text (V106-33: old mentions, highlight phrases and
/// global mentions are all this one rule).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct PhraseRule {
    pub enabled: bool,
    /// Words, or a regex when `regex` is on.
    pub pattern: String,
    pub regex: bool,
    pub flavor: RegexFlavor,
    pub case_sensitive: bool,
    /// Not inside a longer word.
    pub whole_word: bool,
    /// Look-alike letters count (Cyrillic а for a, 𝐟𝐚𝐧𝐜𝐲 text).
    pub lookalikes: bool,
    #[serde(flatten)]
    #[ts(flatten)]
    pub alert: Alert,
}

impl Default for PhraseRule {
    fn default() -> Self {
        // `show` stays off: serde fills missing keys from here, and v1.04 phrases have none.
        // The UI turns it on for a new row (Q12).
        Self {
            enabled: true,
            pattern: String::new(),
            regex: false,
            flavor: RegexFlavor::Rust,
            case_sensitive: false,
            whole_word: true,
            lookalikes: false,
            alert: Alert::default(),
        }
    }
}

impl PhraseRule {
    /// V106-33 What makes two rows "the same" (merge count, duplicate marks, the Chatterino importer).
    /// R1 Case folds only for plain words that ignore case: `\d+` and `\D+` stay apart.
    pub fn dup_key(&self) -> (String, bool, Option<RegexFlavor>, bool) {
        let pattern = self.pattern.trim();
        let pattern = if !self.regex && !self.case_sensitive {
            pattern.to_lowercase()
        } else {
            pattern.to_owned()
        };
        (pattern, self.regex, self.regex.then_some(self.flavor), self.case_sensitive)
    }
}

/// Everything a user sends, by login (or a regex over logins).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct UserRule {
    pub enabled: bool,
    pub login: String,
    pub regex: bool,
    #[serde(flatten)]
    #[ts(flatten)]
    pub alert: Alert,
}

impl Default for UserRule {
    fn default() -> Self {
        Self {
            enabled: true,
            login: String::new(),
            regex: false,
            alert: Alert::default(),
        }
    }
}

/// Senders with a Twitch badge: `moderator`, `subscriber` (any tier), a tier `subscriber1` to
/// `subscriber3` (V105-16), or one exact version like `subscriber/3012`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeRule {
    pub enabled: bool,
    pub badge: String,
    #[serde(flatten)]
    #[ts(flatten)]
    pub alert: Alert,
}

impl Default for BadgeRule {
    fn default() -> Self {
        Self {
            enabled: true,
            badge: String::new(),
            alert: Alert::default(),
        }
    }
}

/// C26 The app a message was sent from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SentFrom {
    #[default]
    Web,
    Android,
    Ios,
    Mobile,
    Odd,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct AppRule {
    pub enabled: bool,
    pub app: SentFrom,
    #[serde(flatten)]
    #[ts(flatten)]
    pub alert: Alert,
}

impl Default for AppRule {
    fn default() -> Self {
        Self {
            enabled: true,
            app: SentFrom::Web,
            alert: Alert::default(),
        }
    }
}

/// F2 Hides a message, or swaps the matching words for `replace`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct IgnoreRule {
    pub enabled: bool,
    pub pattern: String,
    pub regex: bool,
    pub case_sensitive: bool,
    /// Hide the whole message instead of replacing the words.
    pub block: bool,
    /// Regex rules may use `\1`.
    pub replace: String,
    /// F4 A hidden message still goes to this feed tab.
    pub route: String,
}

impl Default for IgnoreRule {
    fn default() -> Self {
        Self {
            enabled: true,
            pattern: String::new(),
            regex: false,
            case_sensitive: false,
            block: false,
            replace: "***".into(),
            route: String::new(),
        }
    }
}

/// F2 Whose messages you see from people your account blocked on Twitch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BlockedShow {
    /// Never shown.
    #[default]
    Never,
    /// Shown where you're a moderator or the broadcaster.
    Moderator,
    /// Shown in your own channel.
    Broadcaster,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Blocked {
    /// Hide people your account blocked on Twitch.
    pub enabled: bool,
    pub show: BlockedShow,
}

impl Default for Blocked {
    fn default() -> Self {
        Self {
            enabled: true,
            show: BlockedShow::Never,
        }
    }
}

/// F3 A named filter in Chatterino's language. Chat panels list the ones they use.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct FilterDef {
    pub id: String,
    pub name: String,
    pub expr: String,
}

/// F6 Near-duplicate messages.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Similarity {
    pub enabled: bool,
    /// Percent alike, more than this counts.
    pub threshold: u8,
    pub max_age_secs: u32,
    /// Earlier messages to compare with.
    pub depth: u8,
    pub same_user: bool,
    /// Check your own messages too.
    pub own: bool,
    /// Hide them instead of greying them out.
    pub hide: bool,
}

impl Default for Similarity {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 90,
            max_age_secs: 5,
            depth: 3,
            same_user: true,
            own: false,
            hide: false,
        }
    }
}

/// C28 Someone hidden only for you.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct HiddenUser {
    pub id: String,
    pub login: String,
}

// V105-12 An empty rule (one just added, being filled in) matches nothing, and goes once
// the settings page is left. One definition for both.
impl PhraseRule {
    pub fn is_blank(&self) -> bool {
        self.pattern.trim().is_empty()
    }
}

impl UserRule {
    pub fn is_blank(&self) -> bool {
        self.login.trim().is_empty()
    }
}

impl BadgeRule {
    pub fn is_blank(&self) -> bool {
        self.badge.trim().is_empty()
    }
}

impl IgnoreRule {
    pub fn is_blank(&self) -> bool {
        self.pattern.trim().is_empty()
    }
}

impl FilterDef {
    pub fn is_blank(&self) -> bool {
        self.name.trim().is_empty() && self.expr.trim().is_empty()
    }
}
