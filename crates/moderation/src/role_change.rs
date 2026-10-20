//! 1.50 Giving and taking mod and VIP: which API a role can use. Twitch lets the
//! broadcaster and lead moderators do it (Twitch Support, 2025/12/18), but Helix only takes
//! the broadcaster's token: a lead mod gets 401 there, so lead mods go through GQL like
//! twitch.tv. A lead mod may wear the plain mod badge, so a mod's attempt goes to Twitch
//! too and Twitch decides.
//! V107-M8 Twitch won't stack the two: making a VIP a mod (or a mod a VIP) is refused
//! (Helix 422 "you must first remove them as a VIP"), so the other role goes first.

use cyex_chat::model::ModRole;

use crate::command::Need;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path {
    /// `POST/DELETE /moderation/moderators` or `/channels/vips`.
    Helix,
    /// `modUser`, `unmodUser`, `grantVIP`, `revokeVIP`.
    Gql,
}

/// Lead mods without a GQL sign-in: Helix won't take their token.
pub const LEAD_NEEDS_GQL: &str = "Lead mods change mods and VIPs with a GQL sign-in. Add one in Settings → Accounts.";

/// Signed in, but neither token can do it.
pub const NO_TOKEN: &str = "This account can’t moderate. Sign in again in Settings → Accounts.";

/// How `role` changes someone's role. `helix_scope`: a Helix token with the matching
/// `channel:manage:*` scope; `gql`: a GQL token. `Err` is the line to show.
pub fn path(role: ModRole, helix_scope: bool, gql: bool) -> Result<Path, &'static str> {
    match role {
        ModRole::Broadcaster if helix_scope => Ok(Path::Helix),
        _ if gql => Ok(Path::Gql),
        ModRole::Broadcaster => Err(NO_TOKEN),
        ModRole::LeadModerator => Err(LEAD_NEEDS_GQL),
        ModRole::Moderator => Err(Need::Lead.refusal()),
    }
}

/// What a refusal from Twitch reads as. `why` is Twitch's error code or message, `twitch`
/// its friendly text. A plain mod (as far as the badges tell) who isn't allowed is told who
/// may; any other reason stays Twitch's.
pub fn refused(role: ModRole, why: &str, twitch: String) -> String {
    let why = why.to_ascii_uppercase();
    let denied = ["FORBIDDEN", "UNAUTHORIZED", "PERMISSION"].iter().any(|w| why.contains(w));
    if role == ModRole::Moderator && denied {
        Need::Lead.refusal().into()
    } else {
        twitch
    }
}

/// V107-M8 What someone holds in a channel, from the badges Twitch says they have there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Holds {
    /// Lead mods count.
    pub moderator: bool,
    pub vip: bool,
}

impl Holds {
    pub fn of<'a>(sets: impl IntoIterator<Item = &'a str>) -> Holds {
        sets.into_iter().fold(Holds::default(), |h, s| Holds {
            moderator: h.moderator || s == "moderator" || s == "lead_moderator",
            vip: h.vip || s == "vip",
        })
    }

    /// Granting `moderator` (else VIP) to someone who holds the other: that goes first.
    pub fn clash(self, moderator: bool) -> bool {
        if moderator { self.vip } else { self.moderator }
    }
}

/// V107-M8 Whether Twitch refused a grant because they hold the other role (GQL codes are
/// read loosely: `TARGET_IS_VIP` for `modUser`, `GRANTEE_IS_MOD` and kin for `grantVIP`).
pub fn clash_code(moderator: bool, code: &str) -> bool {
    let code = code.to_ascii_uppercase();
    !code.contains("ALREADY") && if moderator { code.contains("VIP") } else { code.contains("MOD") }
}

/// S6 A mod or VIP change, as Twitch's mod-action feeds name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoleAction {
    Mod,
    Unmod,
    Vip,
    Unvip,
}

impl RoleAction {
    /// `moderator` (else VIP) given (`on`) or taken.
    pub fn of(moderator: bool, on: bool) -> RoleAction {
        match (moderator, on) {
            (true, true) => RoleAction::Mod,
            (true, false) => RoleAction::Unmod,
            (false, true) => RoleAction::Vip,
            (false, false) => RoleAction::Unvip,
        }
    }

    /// From a feed's action name; `None` for any other action.
    pub fn parse(action: &str) -> Option<RoleAction> {
        Some(match action {
            "mod" => RoleAction::Mod,
            "unmod" => RoleAction::Unmod,
            "vip" => RoleAction::Vip,
            "unvip" => RoleAction::Unvip,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RoleAction::Mod => "mod",
            RoleAction::Unmod => "unmod",
            RoleAction::Vip => "vip",
            RoleAction::Unvip => "unvip",
        }
    }

    /// Whether it is about the mod role (else VIP).
    pub fn moderator(self) -> bool {
        matches!(self, RoleAction::Mod | RoleAction::Unmod)
    }

    /// Whether the role is given (else taken).
    pub fn on(self) -> bool {
        matches!(self, RoleAction::Mod | RoleAction::Vip)
    }
}

/// Twitch's action for a change.
pub fn action(moderator: bool, on: bool) -> RoleAction {
    RoleAction::of(moderator, on)
}

/// The one chat line after a swap.
pub fn swapped(who: &str, moderator: bool) -> String {
    if moderator {
        format!("Removed {who} as VIP and made them a mod.")
    } else {
        format!("Removed {who} as mod and made them a VIP.")
    }
}

/// V107-M8 Whether a VIP grant failed for want of slots: GQL `MAX_VIPS_REACHED`, Helix 409.
/// Helix has no slot count to check first, so a swap only learns it from the grant.
pub fn no_slots(moderator: bool, code: &str, status: u16) -> bool {
    !moderator && (code.eq_ignore_ascii_case("MAX_VIPS_REACHED") || status == 409)
}

/// V107-M8 The grant after taking the other role away failed, so the old role is put back
/// (any reason: nobody should end up with neither). The chat line; `restored`: it is back.
pub fn grant_failed(who: &str, moderator: bool, why: &str, slots: bool, restored: bool) -> String {
    if !restored {
        return half_done(who, moderator, why);
    }
    let (new, old) = if moderator { ("a mod", "VIP") } else { ("VIP", "mod") };
    let why = if slots { "no VIP slots left" } else { why.trim_end_matches('.') };
    format!("Couldn’t make {who} {new}: {why}. Kept them {old}.")
}

/// V107-M8 After a swap's grant failed: the own-echo marks to drop, so those echoes get
/// their line. The grant never happened; the put-back only when it failed. The removal
/// stays marked: the [`grant_failed`] line says it either way.
pub fn unmark_after_failed_grant(moderator: bool, restored: bool) -> Vec<RoleAction> {
    let mut v = vec![RoleAction::of(moderator, true)];
    if !restored {
        v.push(RoleAction::of(!moderator, true));
    }
    v
}

/// The other role went, the grant didn't.
pub fn half_done(who: &str, moderator: bool, why: &str) -> String {
    let why = why.trim_end_matches('.');
    if moderator {
        format!("Removed {who} as VIP, but couldn’t make them a mod: {why}.")
    } else {
        format!("Removed {who} as mod, but couldn’t make them a VIP: {why}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ModRole::{Broadcaster, LeadModerator, Moderator};

    #[test]
    fn v107_m8_holds_from_badges() {
        assert_eq!(Holds::of(["subscriber"]), Holds::default());
        assert_eq!(Holds::of(["lead_moderator"]), Holds { moderator: true, vip: false });
        assert_eq!(Holds::of(["vip", "subscriber"]), Holds { moderator: false, vip: true });
        let vip = Holds::of(["vip"]);
        assert!(vip.clash(true), "mod a VIP: VIP goes first");
        assert!(!vip.clash(false), "already a VIP: Twitch says so");
        let m = Holds::of(["moderator"]);
        assert!(m.clash(false), "VIP a mod: mod goes first");
        assert!(!Holds::default().clash(true));
    }

    #[test]
    fn v107_m8_clash_codes_and_lines() {
        assert!(clash_code(true, "TARGET_IS_VIP"));
        assert!(clash_code(false, "GRANTEE_IS_MOD"));
        assert!(clash_code(false, "grantee_is_moderator"));
        assert!(!clash_code(true, "TARGET_ALREADY_MOD"));
        assert!(!clash_code(false, "GRANTEE_ALREADY_VIP"));
        assert!(!clash_code(false, "MAX_VIPS_REACHED"));
        assert!(!clash_code(true, "FORBIDDEN"));
        assert_eq!(action(true, false), RoleAction::Unmod);
        assert_eq!(action(false, true).as_str(), "vip");
        assert_eq!(swapped("bob", true), "Removed bob as VIP and made them a mod.");
        assert_eq!(swapped("bob", false), "Removed bob as mod and made them a VIP.");
        assert_eq!(half_done("bob", true, "No user called bob."), "Removed bob as VIP, but couldn’t make them a mod: No user called bob.");
    }

    #[test]
    fn v107_m8_failed_grant_keeps_the_removal_quiet() {
        use RoleAction::*;
        // VIP → mod, grant refused, VIP back: the grant's mark goes, unvip and vip stay.
        assert_eq!(unmark_after_failed_grant(true, true), vec![Mod]);
        // Put-back failed too: its mark goes, the removal's stays (the line says it).
        assert_eq!(unmark_after_failed_grant(true, false), vec![Mod, Vip]);
        assert_eq!(unmark_after_failed_grant(false, false), vec![Vip, Mod]);
        assert!(!unmark_after_failed_grant(false, false).contains(&action(true, false)));
    }

    #[test]
    fn s6_role_actions_round_trip() {
        for (m, on) in [(true, true), (true, false), (false, true), (false, false)] {
            let a = RoleAction::of(m, on);
            assert_eq!(RoleAction::parse(a.as_str()), Some(a));
            assert_eq!((a.moderator(), a.on()), (m, on));
        }
        assert_eq!(RoleAction::parse("ban"), None);
        assert_eq!(RoleAction::parse("MOD"), None, "feeds say it lower case");
    }

    #[test]
    fn v107_m8_failed_grant_rolls_back() {
        assert!(no_slots(false, "MAX_VIPS_REACHED", 0));
        assert!(no_slots(false, "max_vips_reached", 0));
        assert!(no_slots(false, "", 409));
        assert!(!no_slots(true, "", 409), "only VIPs have slots");
        assert!(!no_slots(false, "FORBIDDEN", 403));
        assert_eq!(grant_failed("bob", false, "No VIP slots left.", true, true), "Couldn’t make bob VIP: no VIP slots left. Kept them mod.");
        assert_eq!(grant_failed("bob", true, "You can’t do that here.", false, true), "Couldn’t make bob a mod: You can’t do that here. Kept them VIP.");
        // Putting it back failed too: say what's left.
        assert_eq!(grant_failed("bob", false, "No VIP slots left.", true, false), "Removed bob as mod, but couldn’t make them a VIP: No VIP slots left.");
    }

    #[test]
    fn broadcaster_uses_helix_when_it_can() {
        assert_eq!(path(Broadcaster, true, true), Ok(Path::Helix));
        assert_eq!(path(Broadcaster, true, false), Ok(Path::Helix));
        // Signed in without the scope (a pasted token): GQL, else sign in again.
        assert_eq!(path(Broadcaster, false, true), Ok(Path::Gql));
        assert_eq!(path(Broadcaster, false, false), Err(NO_TOKEN));
    }

    #[test]
    fn lead_mods_never_use_helix() {
        // Helix answers a lead mod's token with 401, scope or not.
        assert_eq!(path(LeadModerator, true, true), Ok(Path::Gql));
        assert_eq!(path(LeadModerator, false, true), Ok(Path::Gql));
        assert_eq!(path(LeadModerator, true, false), Err(LEAD_NEEDS_GQL));
        assert_eq!(path(LeadModerator, false, false), Err(LEAD_NEEDS_GQL));
    }

    #[test]
    fn mods_ask_twitch_and_hear_who_may() {
        // A lead mod wearing the plain mod badge looks like a mod: Twitch decides.
        assert_eq!(path(Moderator, true, true), Ok(Path::Gql));
        assert_eq!(path(Moderator, true, false), Err(Need::Lead.refusal()));
        let no = || "You can’t do that here.".to_owned();
        assert_eq!(refused(Moderator, "FORBIDDEN", no()), Need::Lead.refusal());
        assert_eq!(refused(Moderator, "forbidden", no()), Need::Lead.refusal());
        assert_eq!(refused(Moderator, "NO_PERMISSIONS", no()), Need::Lead.refusal());
        // Allowed after all (a lead mod in mod's badge): Twitch's own reason.
        assert_eq!(refused(Moderator, "TARGET_ALREADY_MOD", "bob is already a mod.".into()), "bob is already a mod.");
        assert_eq!(refused(LeadModerator, "FORBIDDEN", no()), no());
        assert_eq!(refused(Broadcaster, "MAX_VIPS_REACHED", "No VIP slots left.".into()), "No VIP slots left.");
    }
}
