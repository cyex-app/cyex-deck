//! The rule types of the highlights partition.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub(super) const MAX_PATTERN_CHARS: usize = 500;
pub(super) const MAX_ROUTE_CHARS: usize = 40;

/// What a matching message does (2.6), shared by every rule.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Alert {
    /// `#rrggbbaa`. Empty uses the highlight's own color.
    pub color: String,
    pub sound: bool,
    pub flash: bool,
    /// F4 Also copy the message into the feed tab with this name. Empty: nowhere.
    pub route: String,
}

impl Alert {
    pub(super) fn sanitize(&mut self) {
        let hex = self.color.strip_prefix('#').unwrap_or("");
        if !(matches!(hex.len(), 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())) {
            self.color.clear();
        }
        self.color.make_ascii_lowercase();
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
                route: String::new(),
            },
        }
    }

    pub(super) const fn off() -> Self {
        let mut b = Self::on();
        b.enabled = false;
        b
    }
}

impl Default for BuiltIn {
    fn default() -> Self {
        Self::on()
    }
}

/// Words or a regex in the message text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct PhraseRule {
    pub enabled: bool,
    pub pattern: String,
    pub regex: bool,
    pub case_sensitive: bool,
    #[serde(flatten)]
    #[ts(flatten)]
    pub alert: Alert,
}

impl Default for PhraseRule {
    fn default() -> Self {
        Self {
            enabled: true,
            pattern: String::new(),
            regex: false,
            case_sensitive: false,
            alert: Alert::default(),
        }
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

/// Senders with a Twitch badge: `moderator`, `subscriber`, or one version like
/// `subscriber/3000`.
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
