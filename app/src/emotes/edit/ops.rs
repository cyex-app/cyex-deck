//! V106-31/32 What the UI sends and gets for 7TV set edits.

use cyex_emotes::seventv::account::EditError;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One change to a 7TV set. Names are exact and case-sensitive.
#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum EditOp {
    #[serde(rename_all = "camelCase")]
    Add {
        set_id: String,
        emote_id: String,
        alias: String,
    },
    #[serde(rename_all = "camelCase")]
    Remove {
        set_id: String,
        emote_id: String,
        alias: String,
    },
    #[serde(rename_all = "camelCase")]
    Rename {
        set_id: String,
        emote_id: String,
        from: String,
        to: String,
    },
}

impl EditOp {
    pub fn set_id(&self) -> &str {
        match self {
            EditOp::Add { set_id, .. } | EditOp::Remove { set_id, .. } | EditOp::Rename { set_id, .. } => {
                set_id
            }
        }
    }

    pub fn emote_id(&self) -> &str {
        match self {
            EditOp::Add { emote_id, .. }
            | EditOp::Remove { emote_id, .. }
            | EditOp::Rename { emote_id, .. } => emote_id,
        }
    }

    /// The name the set holds afterwards (add, rename) or held (remove).
    pub fn name(&self) -> &str {
        match self {
            EditOp::Add { alias, .. } | EditOp::Remove { alias, .. } => alias,
            EditOp::Rename { to, .. } => to,
        }
    }

    /// What undoes it (research §8).
    pub fn undo(&self) -> EditOp {
        match self.clone() {
            EditOp::Add {
                set_id,
                emote_id,
                alias,
            } => EditOp::Remove {
                set_id,
                emote_id,
                alias,
            },
            EditOp::Remove {
                set_id,
                emote_id,
                alias,
            } => EditOp::Add {
                set_id,
                emote_id,
                alias,
            },
            EditOp::Rename {
                set_id,
                emote_id,
                from,
                to,
            } => EditOp::Rename {
                set_id,
                emote_id,
                from: to,
                to: from,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditDone {
    /// Whose emotes changed, display name.
    pub channel: String,
    pub undo: EditOp,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum EditFail {
    SignInAgain,
    NotAllowed,
    RateLimited {
        #[ts(type = "number")]
        seconds: u64,
    },
    Down,
    Full,
    NameTaken,
    Private,
    NotFound,
    BadName,
    NoConsent,
    SignedOut,
    /// A test run without the loopback mock: nothing is sent.
    TestRun,
    Bad,
}

impl From<EditError> for EditFail {
    fn from(e: EditError) -> Self {
        match e {
            EditError::SignInAgain => EditFail::SignInAgain,
            EditError::NotAllowed => EditFail::NotAllowed,
            EditError::RateLimited { seconds } => EditFail::RateLimited { seconds },
            EditError::Down => EditFail::Down,
            EditError::Full => EditFail::Full,
            EditError::NameTaken => EditFail::NameTaken,
            EditError::Private => EditFail::Private,
            EditError::NotFound => EditFail::NotFound,
            EditError::BadName => EditFail::BadName,
            EditError::Bad => EditFail::Bad,
        }
    }
}

/// Whether the account's edit reach is known.
#[derive(Clone, Copy, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Reachability {
    Ready,
    Loading,
    NoConsent,
    SignedOut,
    SignInAgain,
    NoAccount,
    Down,
}

/// Why one channel can't take the emote. TS name: settings already has a `Blocked`.
#[derive(Clone, Copy, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TargetBlocked")]
pub enum Blocked {
    Already,
    Private,
    Full,
}

/// Why no channel can take it.
#[derive(Clone, Copy, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AddBlocked {
    Already,
    Private,
    Nowhere,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddTarget {
    pub set_id: String,
    pub channel: String,
    /// The account's own channel.
    pub own: bool,
    /// This chat's channel.
    pub viewed: bool,
    pub blocked: Option<Blocked>,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddOptions {
    pub blocked: Option<AddBlocked>,
    /// This chat's channel, yours, then A-Z (Q3).
    pub targets: Vec<AddTarget>,
}

/// The emote is in this chat's channel set and the account can edit it (Q6).
#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HereEdit {
    pub set_id: String,
    pub channel: String,
    pub alias: String,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditOptions {
    pub state: Reachability,
    /// The emote's own 7TV name (Q5).
    pub own_name: String,
    pub add: Option<AddOptions>,
    pub here: Option<HereEdit>,
}

#[derive(Clone, Copy, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NameCheck {
    Free,
    Taken,
    /// A rename to the name it has.
    Same,
}
