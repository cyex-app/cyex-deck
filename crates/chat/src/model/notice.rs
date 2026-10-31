//! System lines: subs, gifts, raids, announcements, moderation and errors.

use serde::Serialize;
use ts_rs::TS;

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
}

/// V108-C11 A user named in a notice: the name as shown (may be localized) and the login for the usercard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NoticeUser {
    pub name: String,
    pub login: String,
}

impl NoticeUser {
    pub fn new(name: impl Into<String>, login: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            login: login.into(),
        }
    }
}

/// C37 V108-C03 Who joined or left, as the line shows them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Presence {
    pub joined: bool,
    /// Logins written out, in order of arrival.
    pub names: Vec<String>,
    /// Names past those, only counted.
    pub more: u32,
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
