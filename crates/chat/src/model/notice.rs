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
    pub recipients: Vec<String>,
    /// Announcement color: `primary`, `blue`, `green`, `orange`, `purple`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<String>,
}

impl Notice {
    pub fn new(kind: NoticeKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            recipients: Vec::new(),
            color: None,
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
    /// Timeouts, bans, deletions, chat clears.
    Moderation,
    /// Room modes, joins and other info from Twitch.
    Info,
    /// A message of ours that didn't go through.
    Error,
}
