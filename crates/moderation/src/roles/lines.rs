//! V107-R42 What role changes read as: one table for Twitch's refusal codes (GQL's, and
//! Helix statuses read as them), shared by every role, and the lines after a change.

use cyex_chat::model::ModRole;

use super::slots::{FULL, LOCKED};
use super::{Change, Refusal, Role};
use crate::command::Need;

/// "a mod", "an editor".
pub(super) fn a(role: Role) -> String {
    let r = role.name();
    let an = matches!(r.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u'));
    format!("{} {r}", if an { "an" } else { "a" })
}

/// A step Twitch refused, in a short line. `by` is who tried. Unknown codes show the code.
pub fn refused(code: &str, who: &str, c: Change, by: ModRole) -> String {
    let r = c.role;
    let chat_role = matches!(r, Role::Mod | Role::Vip);
    match code.to_ascii_uppercase().as_str() {
        "TARGET_NOT_FOUND" | "GRANTEE_NOT_FOUND" | "REVOKEE_NOT_FOUND" | "USER_NOT_FOUND" | "TARGET_USER_NOT_FOUND"
        | "TARGET_IS_ANONYMOUS" => format!("No user called {who}."),
        "TARGET_IS_VIP" => format!("{who} is a VIP."),
        "TARGET_IS_MOD" | "TARGET_IS_MODERATOR" | "GRANTEE_IS_MOD" | "GRANTEE_IS_MODERATOR" => format!("{who} is a mod."),
        "CANNOT_BE_VIP_AND_MOD" => format!("{who} can’t be a VIP and a mod."),
        "TARGET_IS_BROADCASTER" => "That’s the broadcaster.".into(),
        "TARGET_IS_STAFF" | "TARGET_IS_ADMIN" | "TARGET_IS_GLOBAL_MOD" => format!("{who} is Twitch staff."),
        "TARGET_IS_SELF" | "CANNOT_ASSIGN_TO_SELF" => "That’s you.".into(),
        "GRANTEE_CHAT_BANNED" | "USER_IS_BANNED" | "TARGET_BANNED" | "TARGET_IS_BANNED" | "GRANTEE_BANNED"
        | "GRANTEE_IS_BANNED" => format!("{who} is banned here. Unban them first."),
        "MAX_VIPS_REACHED" => FULL.into(),
        "VIP_ACHIEVEMENT_INCOMPLETE" => LOCKED.into(),
        "ROLE_COUNT_LIMIT_REACHED" | "MAX_ARTISTS_REACHED" => format!("No {} slots left.", r.name()),
        "TARGET_ALREADY_MOD" | "GRANTEE_ALREADY_VIP" | "ALREADY_ASSIGNED" | "ALREADY_EDITOR" | "TARGET_ALREADY_EDITOR"
        | "GRANTEE_ALREADY_HAS_ROLE" => format!("{who} is already {}.", a(r)),
        "TARGET_NOT_MOD" | "REVOKEE_NOT_VIP" | "NOT_ASSIGNED" | "TARGET_NOT_EDITOR" | "REVOKEE_DOES_NOT_HAVE_ROLE" => {
            format!("{who} isn’t {}.", a(r))
        }
        "CANNOT_BE_ASSIGNED" => format!("{who} can’t be {}.", a(r)),
        c if c.starts_with("CANNOT_ASSIGN") => format!("{who} can’t be {}.", a(r)),
        "RATELIMITED" | "RATE_LIMITED" => "Too many changes at once. Try again in a moment.".into(),
        "PERMISSION_DENIED" | "FORBIDDEN" | "UNAUTHORIZED" | "NO_PERMISSIONS" => match by {
            // A plain mod (as far as the badges tell) is told who may.
            ModRole::Moderator if chat_role => Need::Lead.refusal().into(),
            ModRole::Broadcaster => "You can’t do that here.".into(),
            _ if chat_role => "You can’t do that here.".into(),
            _ => Need::Broadcaster.refusal().into(),
        },
        cyex_twitch::gql::dashboard::NOT_DONE => format!("Twitch didn’t {} it. Try again.", if c.on { "do" } else { "undo" }),
        other => format!("Twitch refused it ({other})."),
    }
}

/// [`refused`] for any refusal.
pub(super) fn line(r: &Refusal, who: &str, c: Change, by: ModRole) -> String {
    match r {
        Refusal::Code(code) => refused(code, who, c, by),
        Refusal::Line(l) => l.clone(),
    }
}

/// A refusal as a clause inside another line: "no VIP slots left".
pub(super) fn clause(r: &Refusal, who: &str, c: Change, by: ModRole) -> String {
    if let Refusal::Code(code) = r {
        match code.to_ascii_uppercase().as_str() {
            "MAX_VIPS_REACHED" => return "no VIP slots left".into(),
            "VIP_ACHIEVEMENT_INCOMPLETE" => return "VIPs aren’t unlocked yet".into(),
            _ => {}
        }
    }
    line(r, who, c, by).trim_end_matches('.').to_owned()
}

/// The line after one change went through.
pub(super) fn done(who: &str, c: Change) -> String {
    if c.on { format!("Made {who} {}.", a(c.role)) } else { format!("{who} is no longer {}.", a(c.role)) }
}

/// V107-M8 The other role went first, then the grant went through.
pub(super) fn swapped(who: &str, old: Role, new: Role) -> String {
    format!("Removed {who} as {} and made them {}.", old.name(), a(new))
}

/// The grant after taking `old` failed, and `old` is back.
pub(super) fn kept(who: &str, old: Role, new: Role, why: &str) -> String {
    format!("Couldn’t make {who} {}: {why}. Kept them {}.", a(new), old.name())
}

/// The grant after taking `old` failed, and putting `old` back failed too.
pub(super) fn half_done(who: &str, old: Role, new: Role, why: &str) -> String {
    format!("Removed {who} as {}, but couldn’t make them {}: {why}.", old.name(), a(new))
}

fn list(roles: &[Role], last: &str) -> String {
    let names: Vec<String> = roles.iter().map(|r| a(*r)).collect();
    match names.split_last() {
        None => String::new(),
        Some((one, [])) => one.clone(),
        Some((end, rest)) => format!("{} {last} {end}", rest.join(", ")),
    }
}

/// The line after several changes went through.
pub(super) fn summary(who: &str, changes: &[Change]) -> String {
    let given: Vec<Role> = changes.iter().filter(|c| c.on).map(|c| c.role).collect();
    let taken: Vec<Role> = changes.iter().filter(|c| !c.on).map(|c| c.role).collect();
    match (given.is_empty(), taken.is_empty()) {
        (false, true) => format!("Made {who} {}.", list(&given, "and")),
        (true, false) => format!("{who} is no longer {}.", list(&taken, "or")),
        _ => format!("Changed {who}’s roles."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ModRole::{Broadcaster, LeadModerator, Moderator};
    use Role::*;

    fn r(code: &str, role: Role, on: bool) -> String {
        refused(code, "bob", Change::new(role, on), Broadcaster)
    }

    #[test]
    fn v107_r42_every_refusal_line() {
        let cases: &[(&str, Role, bool, &str)] = &[
            ("TARGET_NOT_FOUND", Mod, true, "No user called bob."),
            ("GRANTEE_NOT_FOUND", Vip, true, "No user called bob."),
            ("REVOKEE_NOT_FOUND", Vip, false, "No user called bob."),
            ("USER_NOT_FOUND", Editor, true, "No user called bob."),
            ("TARGET_USER_NOT_FOUND", Artist, true, "No user called bob."),
            ("TARGET_IS_ANONYMOUS", Mod, true, "No user called bob."),
            ("TARGET_IS_VIP", Mod, true, "bob is a VIP."),
            ("TARGET_IS_MOD", Vip, true, "bob is a mod."),
            ("GRANTEE_IS_MODERATOR", Vip, true, "bob is a mod."),
            ("CANNOT_BE_VIP_AND_MOD", Vip, true, "bob can’t be a VIP and a mod."),
            ("TARGET_IS_BROADCASTER", Mod, true, "That’s the broadcaster."),
            ("TARGET_IS_STAFF", Mod, true, "bob is Twitch staff."),
            ("TARGET_IS_SELF", Vip, true, "That’s you."),
            ("CANNOT_ASSIGN_TO_SELF", LeadMod, true, "That’s you."),
            ("GRANTEE_CHAT_BANNED", Vip, true, "bob is banned here. Unban them first."),
            ("USER_IS_BANNED", LeadMod, true, "bob is banned here. Unban them first."),
            ("TARGET_IS_BANNED", Mod, true, "bob is banned here. Unban them first."),
            ("MAX_VIPS_REACHED", Vip, true, FULL),
            ("VIP_ACHIEVEMENT_INCOMPLETE", Vip, true, LOCKED),
            ("ROLE_COUNT_LIMIT_REACHED", LeadMod, true, "No lead mod slots left."),
            ("ROLE_COUNT_LIMIT_REACHED", Agent, true, "No agent slots left."),
            ("MAX_ARTISTS_REACHED", Artist, true, "No artist slots left."),
            ("TARGET_ALREADY_MOD", Mod, true, "bob is already a mod."),
            ("GRANTEE_ALREADY_VIP", Vip, true, "bob is already a VIP."),
            ("ALREADY_ASSIGNED", LeadMod, true, "bob is already a lead mod."),
            ("ALREADY_EDITOR", Editor, true, "bob is already an editor."),
            ("GRANTEE_ALREADY_HAS_ROLE", Artist, true, "bob is already an artist."),
            ("TARGET_NOT_MOD", Mod, false, "bob isn’t a mod."),
            ("REVOKEE_NOT_VIP", Vip, false, "bob isn’t a VIP."),
            ("NOT_ASSIGNED", LeadMod, false, "bob isn’t a lead mod."),
            ("TARGET_NOT_EDITOR", Editor, false, "bob isn’t an editor."),
            ("REVOKEE_DOES_NOT_HAVE_ROLE", Artist, false, "bob isn’t an artist."),
            ("CANNOT_BE_ASSIGNED", Agent, true, "bob can’t be an agent."),
            ("CANNOT_ASSIGN_ROLE", LeadMod, true, "bob can’t be a lead mod."),
            ("RATELIMITED", Mod, true, "Too many changes at once. Try again in a moment."),
            ("rate_limited", Editor, true, "Too many changes at once. Try again in a moment."),
            ("FORBIDDEN", Mod, true, "You can’t do that here."),
            ("NOT_DONE", Editor, true, "Twitch didn’t do it. Try again."),
            ("NOT_DONE", Editor, false, "Twitch didn’t undo it. Try again."),
            ("SOMETHING_NEW", Mod, false, "Twitch refused it (SOMETHING_NEW)."),
        ];
        for (code, role, on, want) in cases {
            assert_eq!(r(code, *role, *on), *want, "{code} {role:?}");
        }
    }

    #[test]
    fn v107_r42_permission_lines_by_who_tried() {
        let mod_ = Change::new(Mod, true);
        let lead = Change::new(LeadMod, true);
        assert_eq!(refused("FORBIDDEN", "bob", mod_, Moderator), Need::Lead.refusal());
        assert_eq!(refused("no_permissions", "bob", mod_, Moderator), Need::Lead.refusal());
        assert_eq!(refused("FORBIDDEN", "bob", mod_, LeadModerator), "You can’t do that here.");
        assert_eq!(refused("PERMISSION_DENIED", "bob", lead, LeadModerator), "Only the broadcaster can do that.");
        assert_eq!(refused("PERMISSION_DENIED", "bob", lead, Moderator), "Only the broadcaster can do that.");
        // Allowed after all (a lead mod in mod's badge): Twitch's own reason.
        assert_eq!(refused("TARGET_ALREADY_MOD", "bob", mod_, Moderator), "bob is already a mod.");
    }

    #[test]
    fn v107_r42_change_lines() {
        assert_eq!(done("bob", Change::new(Artist, true)), "Made bob an artist.");
        assert_eq!(done("bob", Change::new(Vip, false)), "bob is no longer a VIP.");
        assert_eq!(done("bob", Change::new(LeadMod, true)), "Made bob a lead mod.");
        assert_eq!(swapped("bob", Vip, Mod), "Removed bob as VIP and made them a mod.");
        assert_eq!(swapped("bob", Mod, Vip), "Removed bob as mod and made them a VIP.");
        assert_eq!(kept("bob", Mod, Vip, "no VIP slots left"), "Couldn’t make bob a VIP: no VIP slots left. Kept them mod.");
        assert_eq!(half_done("bob", Vip, Mod, "No user called bob"), "Removed bob as VIP, but couldn’t make them a mod: No user called bob.");
        let on = |r| Change::new(r, true);
        assert_eq!(summary("bob", &[on(Editor), on(Artist)]), "Made bob an editor and an artist.");
        assert_eq!(summary("bob", &[on(Mod), on(Editor), on(Agent)]), "Made bob a mod, an editor and an agent.");
        assert_eq!(summary("bob", &[Change::new(Editor, false), Change::new(Vip, false)]), "bob is no longer an editor or a VIP.");
        assert_eq!(summary("bob", &[on(Editor), Change::new(Vip, false)]), "Changed bob’s roles.");
        let full = Refusal::Code("MAX_VIPS_REACHED".into());
        assert_eq!(clause(&full, "bob", on(Vip), Broadcaster), "no VIP slots left");
        assert_eq!(clause(&Refusal::Line("Twitch is down.".into()), "bob", on(Vip), Broadcaster), "Twitch is down");
    }
}
