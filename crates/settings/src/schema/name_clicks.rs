//! V110-M20 What clicking a name in chat does, per click kind.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One thing a click on a name can do. Timeout and ban need moderator rights in that chat;
/// elsewhere they open the usercard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NameClick {
    #[default]
    Usercard,
    Reply,
    /// Puts `@name` in the message box.
    Mention,
    Copy,
    /// The first timeout length of mod mode.
    Timeout,
    /// Asks first.
    Ban,
    Logs,
    None,
}

/// Plain, middle, Ctrl and Shift click. A plain click is never `None`, a name has to open something.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct NameClicks {
    pub plain: NameClick,
    pub middle: NameClick,
    pub ctrl: NameClick,
    pub shift: NameClick,
}

impl Default for NameClicks {
    /// Today's behaviour: the usercard for a click with or without Ctrl or Shift, nothing for the middle one.
    fn default() -> Self {
        Self {
            plain: NameClick::Usercard,
            middle: NameClick::None,
            ctrl: NameClick::Usercard,
            shift: NameClick::Usercard,
        }
    }
}

impl NameClicks {
    pub(super) fn sanitize(&mut self) {
        if self.plain == NameClick::None {
            self.plain = NameClick::Usercard;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_files_and_partial_ones_load() {
        let c: NameClicks = serde_json::from_str(r#"{"middle":"copy"}"#).unwrap();
        assert_eq!((c.plain, c.middle, c.ctrl), (NameClick::Usercard, NameClick::Copy, NameClick::Usercard));
    }

    #[test]
    fn a_plain_click_keeps_something_to_do() {
        let mut c = NameClicks { plain: NameClick::None, ..NameClicks::default() };
        c.sanitize();
        assert_eq!(c.plain, NameClick::Usercard);
    }
}
