//! System lines: subs, gifts, raids, announcements, moderation and errors.

use serde::Serialize;
use ts_rs::TS;

use super::fragment::Provider;
use super::message::NameColor;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Notice {
    pub kind: NoticeKind,
    /// Ready to show, e.g. "Foo subscribed for 12 months."
    pub text: String,
    /// C16 Gift recipients merged into one line.
    pub recipients: Vec<NoticeUser>,
    /// V108-C11 Names inside `text` that open their usercard (the gifter, a lone recipient).
    pub links: Vec<NoticeUser>,
    /// Announcement color: `primary`, `blue`, `green`, `orange`, `purple`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<String>,
    /// C37 V108-C03 A joins or leaves line's names, each opening its usercard.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub presence: Option<Presence>,
    /// V111-01 An emote-change line's emote, drawn as a small picture after its name in `text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub emote: Option<NoticeEmote>,
    /// V111-17 A message AutoMod holds, with its state: the line shows Approve and Deny (mods only).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub held: Option<HeldNote>,
    /// V111-17 A button after the text, e.g. "Sign in again".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub action: Option<NoticeAction>,
    /// V111-30 A send of ours that didn't go through: its text, for Copy and Retry.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unsent: Option<Unsent>,
}

/// V111-30 The line a "Not sent" notice is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Unsent {
    /// As it was sent, `/me ` in front of an action: Retry sends it as is.
    pub text: String,
    /// The message it answered, for a retry.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reply_to: Option<String>,
    /// Trying again can help (a wait, a lost reply); not after a ban or an AutoMod hold.
    pub retry: bool,
}

/// V111-17 What a notice's button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NoticeAction {
    /// Opens Settings → Accounts to sign the account in again.
    SignIn,
}

/// 1.56, V106-07 A held message's state, in chat, the mod view and the AutoMod feed alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HeldStatus {
    Pending,
    Allowed,
    Denied,
    Expired,
}

impl HeldStatus {
    /// Twitch's word for it: EventSub `Approved`/`Denied`/`Expired`, Hermes `ALLOWED`/`DENIED`/`EXPIRED`.
    pub fn of(s: &str) -> HeldStatus {
        match s.to_ascii_lowercase().as_str() {
            "approved" | "allowed" => HeldStatus::Allowed,
            "denied" => HeldStatus::Denied,
            "pending" => HeldStatus::Pending,
            _ => HeldStatus::Expired,
        }
    }
}

/// V111-17 A message AutoMod holds, as a chat line for the channel's mods.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HeldNote {
    pub message_id: String,
    /// Who sent it (their name opens their usercard).
    pub user: NoticeUser,
    pub text: String,
    /// V108-N15 Wire form: `automod:<category>[:<level>]`, `blocked_term[:<text>]` or `blocked_link`.
    pub reason: String,
    pub status: HeldStatus,
    /// The mod who approved or denied it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub by: Option<String>,
}

/// V111-01 The emote an emote-change line is about (added, removed or renamed to).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NoticeEmote {
    pub provider: Provider,
    pub id: String,
    pub name: String,
    pub animated: bool,
    /// At 1x in px, when the provider says.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub size: Option<(u16, u16)>,
}

/// V108-C11 A user named in a notice: the name as shown (may be localized) and the login for the usercard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NoticeUser {
    pub name: String,
    pub login: String,
    /// V111-16 Their chat colour, when a line of theirs has been seen in the room.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<NameColor>,
}

impl NoticeUser {
    pub fn new(name: impl Into<String>, login: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            login: login.into(),
            color: None,
        }
    }
}

/// C37 V108-C03 Who joined or left, as the line shows them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Presence {
    pub joined: bool,
    /// Logins in order of arrival; all of them (V111-07).
    pub names: Vec<String>,
}

impl Notice {
    pub fn new(kind: NoticeKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            recipients: Vec::new(),
            links: Vec::new(),
            color: None,
            presence: None,
            emote: None,
            held: None,
            action: None,
            unsent: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NoticeKind {
    Sub,
    Gift,
    Raid,
    Announcement,
    Bits,
    /// Watch streaks and other viewer milestones.
    Milestone,
    /// 1.9 A channel point reward redeemed without a message (Hermes).
    Redemption,
    /// Timeouts, bans, deletions, chat clears.
    Moderation,
    /// Room modes, joins and other info from Twitch.
    Info,
    /// A message of ours that didn't go through.
    Error,
}
