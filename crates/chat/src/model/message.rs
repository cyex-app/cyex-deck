//! A chat line as the UI gets it (ARCHITECTURE §4).

use serde::Serialize;
use ts_rs::TS;

use super::{Fragment, Notice};

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatMessage {
    /// Twitch's message id, or a local one (`local-…`) for system lines.
    pub id: String,
    /// Unix milliseconds.
    #[ts(type = "number")]
    pub ts: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sender: Option<Sender>,
    pub body: Vec<Fragment>,
    /// Subs, raids, announcements, timeouts and other system lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub notice: Option<Notice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reply: Option<Reply>,
    /// 1.9 Channel point reward redeemed with this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reward: Option<RewardRef>,
    /// Shared chat: the channel this message was sent in.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source: Option<SharedSource>,
    pub flags: Vec<Flag>,
    #[serde(flatten)]
    #[ts(flatten)]
    pub meta: Meta,
}

/// What phase 8 adds after a message is built.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Meta {
    /// 2.6 Why the line stands out, and in which color.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub highlight: Option<Highlight>,
    /// C2 How often the sender has said this in a row.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub repeat: Option<u32>,
    /// C26 The app it was sent from, read from its nonce.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub client: Option<Client>,
    /// C17
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub translation: Option<Translation>,
    /// 1.42 A message effect power-up: `simmer`, `rainbow-eclipse`, `cosmic-abyss`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub effect: Option<String>,
    /// 1.26 Who an own whisper went to (login), for inline whisper lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub whisper_to: Option<String>,
    /// S7 A line a plugin made.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub plugin: Option<PluginTag>,
}

/// S7 Which plugin made a line, for its mark.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginTag {
    pub id: String,
    pub name: String,
}

/// 2.6
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Highlight {
    pub kind: HighlightKind,
    /// `#rrggbbaa`; `None` uses the kind's own color.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HighlightKind {
    /// Your name, or a reply to you.
    Mention,
    /// A phrase, user, badge or app rule of yours.
    Rule,
    FirstMessage,
    Redeemed,
    Sub,
    Announcement,
    WatchStreak,
    /// Your own messages.
    Own,
}

/// C26
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Client {
    Web,
    Android,
    Ios,
    /// Twitch's newer mobile app.
    Mobile,
    /// A nonce no Twitch app sends.
    Odd,
}

impl Client {
    /// Technorino's rules: 32 lowercase hex is the website, a UUID is iOS (uppercase) or
    /// Android (lowercase), 10 to 12 lowercase letters and digits the new mobile app.
    /// No nonce (Chatterino and most bots) is unknown.
    pub fn from_nonce(nonce: &str) -> Option<Self> {
        let b = nonce.as_bytes();
        let lower_alnum = |c: &u8| c.is_ascii_lowercase() || c.is_ascii_digit();
        Some(match b.len() {
            0 => return None,
            10..=12 if b.iter().all(lower_alnum) => Client::Mobile,
            32 if b.iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c)) => Client::Web,
            32 => return None,
            36 if [8, 13, 18, 23].iter().all(|i| b[*i] == b'-') && b[14] == b'4' => {
                let hex: Vec<u8> = b.iter().copied().filter(|c| *c != b'-').collect();
                let upper = hex.iter().any(|c| (b'A'..=b'F').contains(c));
                let lower = hex.iter().any(|c| (b'a'..=b'f').contains(c));
                let valid = hex.iter().all(u8::is_ascii_hexdigit);
                match (valid, upper, lower) {
                    (true, true, false) => Client::Ios,
                    (true, false, true) => Client::Android,
                    _ => Client::Odd,
                }
            }
            _ => Client::Odd,
        })
    }
}

/// C17
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Translation {
    pub text: String,
    /// Language it was in, e.g. `de`.
    pub from: String,
}

impl ChatMessage {
    pub fn has(&self, flag: Flag) -> bool {
        self.flags.contains(&flag)
    }

    pub fn set(&mut self, flag: Flag) {
        if !self.has(flag) {
            self.flags.push(flag);
        }
    }

    pub fn sender_id(&self) -> Option<&str> {
        self.sender.as_ref().map(|s| s.id.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sender {
    pub id: String,
    pub login: String,
    /// May be localized (C20).
    pub display: String,
    /// C8 Name color made readable on each theme, `#rrggbb`.
    pub color: NameColor,
    pub badges: Vec<Badge>,
}

impl Sender {
    /// C20 The display name isn't just the login with different casing.
    pub fn localized(&self) -> bool {
        !self.display.eq_ignore_ascii_case(&self.login)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NameColor {
    pub dark: String,
    pub light: String,
}

/// Twitch badge `set/version`. Images come from the room's badge catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Badge {
    pub set: String,
    pub version: String,
    /// From `badge-info`: sub months, prediction choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub info: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Reply {
    pub parent_id: String,
    pub parent_login: String,
    pub parent_display: String,
    pub parent_body: String,
    /// The first message of the thread.
    pub thread_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RewardRef {
    /// Custom reward id, or the automatic reward's type.
    pub id: String,
    /// Filled in from the channel's reward list when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cost: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SharedSource {
    pub room_id: String,
    pub badges: Vec<Badge>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Flag {
    /// `/me`
    Action,
    /// Mentions or replies to the signed-in account.
    Mention,
    /// Sent by the signed-in account.
    Own,
    /// C1 Loaded from history when joining.
    History,
    /// C14 Deleted by a mod, timed out or banned.
    Deleted,
    FirstMessage,
    ReturningChatter,
    /// 1.9 "Highlight my message" reward.
    Highlighted,
    /// "Send a message in sub-only mode" reward.
    SkipSubsMode,
    /// 1.42 Power-ups.
    Gigantified,
    Animated,
    /// Paid pinned message (Hype Chat).
    Paid,
    /// F6 Close to one of the sender's last few messages.
    Similar,
    /// 1.26 A whisper, in the whispers view or inline in chat.
    Whisper,
}

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn clients_from_nonces() {
        assert_eq!(Client::from_nonce(""), None);
        assert_eq!(Client::from_nonce("0123456789abcdef0123456789abcdef"), Some(Client::Web));
        assert_eq!(Client::from_nonce("A1B2C3D4-E5F6-4A7B-8C9D-0E1F2A3B4C5D"), Some(Client::Ios));
        assert_eq!(Client::from_nonce("a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d"), Some(Client::Android));
        assert_eq!(Client::from_nonce("abc123def4"), Some(Client::Mobile));
        assert_eq!(Client::from_nonce("hello"), Some(Client::Odd));
    }
}
