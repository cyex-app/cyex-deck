//! V107-R14 Who may do what: the one answer for every mod control. Which actions the
//! menus, the line buttons and the pin banner offer the acting account, and which ones
//! Twitch refuses, for this target and this message. The UI gets the whole answer with
//! the role ([`Caps`] in each `RoleEntry`), so menus stay synchronous: it only sorts the
//! line into a [`Whose`] and picks a cell.
//!
//! The delete rule is Helix "Delete Chat Messages" (under 6 hours, not the broadcaster's,
//! not another mod's) plus what GQL did on 2026/10/19 (the broadcaster deleted their own
//! line). Lead mods change mods and VIPs (1.50, over GQL: `roles`).

use cyex_chat::model::ModRole;
use serde::Serialize;
use ts_rs::TS;

use crate::command::Need;

/// Twitch deletes messages up to 6 hours old.
pub const DELETE_WINDOW_SECS: u32 = 6 * 60 * 60;

/// What a line or a user is to the acting account. Sorted in this order of precedence:
/// the channel itself (by login, or a broadcaster badge), the account's own, someone with
/// a mod badge, anyone else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whose {
    Anyone,
    Own,
    Mod,
    Broadcaster,
}

/// The actions a mod gets on a line or a user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Time out, warn, unban, ban, monitor, restrict (1.49, M3).
    User,
    /// 1.9 Pin the message.
    Pin,
    Delete,
    /// V101-33 Mod mode's buttons in front of the line (M10).
    Quick,
}

/// One cell of the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[ts(export)]
pub struct Cell {
    /// Shows in the menu or on the line.
    pub offered: bool,
    /// Twitch's refusal, where the rule is known (deletes): a short line for a tooltip.
    #[ts(type = "string | null")]
    pub why: Option<&'static str>,
}

/// The delete rule. `old`: the message is over [`DELETE_WINDOW_SECS`] old.
pub fn delete(actor: Option<ModRole>, whose: Whose, old: bool) -> Result<(), &'static str> {
    if actor.is_none() {
        return Err("Only mods can delete messages.");
    }
    if old {
        return Err("Twitch only deletes messages from the last 6 hours.");
    }
    match (actor, whose) {
        (Some(ModRole::Broadcaster), _) | (_, Whose::Anyone | Whose::Own) => Ok(()),
        (_, Whose::Broadcaster) => Err("Mods can’t delete the broadcaster’s messages."),
        (_, Whose::Mod) => Err("Mods can’t delete other mods’ messages."),
    }
}

/// Whether `actor` gets `act` on a line or user that is `whose`, and Twitch's refusal.
pub fn cell(actor: Option<ModRole>, act: Act, whose: Whose, old: bool) -> Cell {
    let Some(role) = actor else {
        let why = if act == Act::Delete {
            delete(actor, whose, old).err()
        } else {
            Some(Need::Mod.refusal())
        };
        return Cell { offered: false, why };
    };
    let not_channel = whose != Whose::Broadcaster;
    match act {
        Act::User => Cell {
            offered: not_channel,
            why: None,
        },
        Act::Pin => Cell {
            offered: true,
            why: None,
        },
        // Delete shows on everyone's lines but the broadcaster's, where Twitch decides; on
        // the broadcaster's only when the rule lets it through.
        Act::Delete => {
            let rule = delete(actor, whose, old);
            Cell {
                offered: not_channel || rule.is_ok(),
                why: rule.err(),
            }
        }
        // Not the broadcaster's, not your own, other mods' for the broadcaster and lead mods
        // (V107-R22, the user's ask; Twitch answers if it refuses one).
        Act::Quick => Cell {
            offered: match whose {
                Whose::Anyone => true,
                Whose::Mod => matches!(role, ModRole::Broadcaster | ModRole::LeadModerator),
                Whose::Own | Whose::Broadcaster => false,
            },
            why: None,
        },
    }
}

/// A cell for a fresh message and one over the delete window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[ts(export)]
pub struct ByAge {
    pub fresh: Cell,
    pub old: Cell,
}

/// One action's cells, by [`Whose`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[ts(export)]
pub struct ByWhose {
    pub anyone: ByAge,
    pub own: ByAge,
    #[serde(rename = "mod")]
    #[ts(rename = "mod")]
    pub moderator: ByAge,
    pub broadcaster: ByAge,
}

/// The whole answer for one role, shipped with it to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Caps {
    /// 1.50 Mod and VIP changes are offered: lead mods and the broadcaster.
    pub roles: bool,
    /// The broadcaster's own commands are offered (listing roles, raids, ads, markers).
    pub broadcaster: bool,
    /// [`DELETE_WINDOW_SECS`], so the UI can tell fresh from old.
    pub window_secs: u32,
    pub user: ByWhose,
    pub pin: ByWhose,
    pub delete: ByWhose,
    pub quick: ByWhose,
}

impl Caps {
    pub fn of(role: ModRole) -> Caps {
        let actor = Some(role);
        let by_whose = |act| {
            let age = |whose| ByAge {
                fresh: cell(actor, act, whose, false),
                old: cell(actor, act, whose, true),
            };
            ByWhose {
                anyone: age(Whose::Anyone),
                own: age(Whose::Own),
                moderator: age(Whose::Mod),
                broadcaster: age(Whose::Broadcaster),
            }
        };
        Caps {
            roles: Need::Lead.allows(actor),
            broadcaster: Need::Broadcaster.allows(actor),
            window_secs: DELETE_WINDOW_SECS,
            user: by_whose(Act::User),
            pin: by_whose(Act::Pin),
            delete: by_whose(Act::Delete),
            quick: by_whose(Act::Quick),
        }
    }

    /// The cell the UI reads for `act` (the same lookup `ui/src/features/modview/may.ts` does).
    pub fn get(&self, act: Act, whose: Whose, old: bool) -> Cell {
        let by = match act {
            Act::User => &self.user,
            Act::Pin => &self.pin,
            Act::Delete => &self.delete,
            Act::Quick => &self.quick,
        };
        let age = match whose {
            Whose::Anyone => &by.anyone,
            Whose::Own => &by.own,
            Whose::Mod => &by.moderator,
            Whose::Broadcaster => &by.broadcaster,
        };
        if old { age.old } else { age.fresh }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Act::*;
    use Whose::{Anyone, Broadcaster as Chan, Mod, Own};

    const B: Option<ModRole> = Some(ModRole::Broadcaster);
    const L: Option<ModRole> = Some(ModRole::LeadModerator);
    const M: Option<ModRole> = Some(ModRole::Moderator);
    const OLD: &str = "Twitch only deletes messages from the last 6 hours.";
    const CHAN: &str = "Mods can’t delete the broadcaster’s messages.";
    const MODS: &str = "Mods can’t delete other mods’ messages.";

    /// V107-R14 Role × action × target × message age: what's offered and what Twitch
    /// refuses, the same as the menus did before (MessageList, nameMenu, lineMod,
    /// deleteRule).
    #[test]
    fn v107_r14_role_action_target_age() {
        type Row = (Option<ModRole>, Act, Whose, bool, bool, Option<&'static str>);
        #[rustfmt::skip]
        let table: &[Row] = &[
            // actor, act, whose, old, offered, why
            (None, User, Anyone, false, false, Some("You’re not a moderator here.")),
            (None, Delete, Own, false, false, Some("Only mods can delete messages.")),
            (None, Quick, Anyone, false, false, Some("You’re not a moderator here.")),
            // User actions: everyone but the channel itself, your own name too.
            (M, User, Anyone, false, true, None), (M, User, Own, false, true, None),
            (M, User, Mod, false, true, None), (M, User, Chan, false, false, None),
            (L, User, Chan, true, false, None), (B, User, Chan, false, false, None),
            (B, User, Mod, true, true, None),
            // Pins: any line, any age.
            (M, Pin, Chan, false, true, None), (M, Pin, Mod, true, true, None), (B, Pin, Own, true, true, None),
            // Deletes, fresh.
            (M, Delete, Anyone, false, true, None), (M, Delete, Own, false, true, None),
            (M, Delete, Mod, false, true, Some(MODS)), (L, Delete, Mod, false, true, Some(MODS)),
            (M, Delete, Chan, false, false, Some(CHAN)), (L, Delete, Chan, false, false, Some(CHAN)),
            (B, Delete, Anyone, false, true, None), (B, Delete, Mod, false, true, None),
            (B, Delete, Chan, false, true, None), (B, Delete, Own, false, true, None),
            // Deletes, over 6 hours: refused for all; still offered where Twitch decides.
            (M, Delete, Anyone, true, true, Some(OLD)), (M, Delete, Chan, true, false, Some(OLD)),
            (B, Delete, Chan, true, false, Some(OLD)), (B, Delete, Anyone, true, true, Some(OLD)),
            // Mod mode's buttons: not the channel, not yourself, mods for the broadcaster and lead mods.
            (M, Quick, Anyone, false, true, None), (M, Quick, Mod, false, false, None),
            (L, Quick, Mod, false, true, None), (B, Quick, Mod, false, true, None),
            (B, Quick, Own, false, false, None), (B, Quick, Chan, false, false, None),
            (M, Quick, Own, true, false, None),
        ];
        for &(actor, act, whose, old, offered, why) in table {
            let c = cell(actor, act, whose, old);
            assert_eq!(
                (c.offered, c.why),
                (offered, why),
                "{actor:?} {act:?} {whose:?} old={old}"
            );
            if let Some(role) = actor {
                assert_eq!(Caps::of(role).get(act, whose, old), c, "the shipped table agrees");
            }
        }
    }

    #[test]
    fn v107_r14_roles_offered_to_lead_mods_and_the_broadcaster() {
        let caps = |r| Caps::of(r);
        assert!(
            !caps(ModRole::Moderator).roles
                && caps(ModRole::LeadModerator).roles
                && caps(ModRole::Broadcaster).roles
        );
        assert!(!caps(ModRole::LeadModerator).broadcaster && caps(ModRole::Broadcaster).broadcaster);
        assert_eq!(caps(ModRole::Moderator).window_secs, 21_600);
    }

    /// The UI tests read Rust's answers from here (`ui/src/features/modview/*.test.ts`).
    #[test]
    fn v107_r14_writes_the_ui_fixture() {
        let Ok(dir) = std::env::var("TS_RS_EXPORT_DIR") else {
            return;
        };
        let all: serde_json::Map<String, serde_json::Value> =
            [ModRole::Moderator, ModRole::LeadModerator, ModRole::Broadcaster]
                .into_iter()
                .map(|r| {
                    (
                        serde_json::to_value(r).unwrap().as_str().unwrap().to_owned(),
                        serde_json::to_value(Caps::of(r)).unwrap(),
                    )
                })
                .collect();
        let text = serde_json::to_string_pretty(&all).unwrap() + "\n";
        let path = std::path::Path::new(&dir).join("caps.fixture.json");
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(path, text).unwrap();
        }
    }

    #[test]
    fn v107_r14_caps_reach_the_ui_in_camel_case() {
        let v = serde_json::to_value(Caps::of(ModRole::Moderator)).unwrap();
        assert_eq!(v["windowSecs"], 21_600);
        assert_eq!(v["delete"]["mod"]["fresh"]["why"], MODS);
        assert_eq!(v["delete"]["broadcaster"]["fresh"]["offered"], false);
        assert!(v["pin"]["anyone"]["old"]["why"].is_null());
    }
}
