//! 1.50 Giving and taking mod and VIP: which API a role can use. Twitch lets the
//! broadcaster and lead moderators do it (Twitch Support, 2025/12/18), but Helix only takes
//! the broadcaster's token: a lead mod gets 401 there, so lead mods go through GQL like
//! twitch.tv. A lead mod may wear the plain mod badge, so a mod's attempt goes to Twitch
//! too and Twitch decides.
//! V107-M8 Twitch won't stack the two: making a VIP a mod (or a mod a VIP) is refused
//! (Helix 422 "you must first remove them as a VIP"), so the other role goes first.
//! V107-R14 The whole sequence ([`set_role`]) runs here against a small port ([`RoleOps`]);
//! the app is the Helix/GQL adapter.

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


/// A refused role change: the line to show, and what Twitch said about it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refused {
    pub text: String,
    /// V107-M8 They hold the other role.
    pub clash: bool,
    /// V107-M8 A VIP grant with no slots left.
    pub slots: bool,
}

impl From<&str> for Refused {
    fn from(text: &str) -> Self {
        text.to_owned().into()
    }
}

impl From<String> for Refused {
    fn from(text: String) -> Self {
        Refused { text, ..Refused::default() }
    }
}

/// V107-R14 What the role change sequence needs from the app, for one account in one
/// channel: Twitch (Helix or GQL), the own-echo marks of the mod feeds, the role buttons and
/// the chat. The app's adapter is `app/src/moderation/run.rs`.
pub trait RoleOps: Sync {
    /// What `who` holds now, from Twitch's badges; `None` when Twitch can't say.
    fn holds(&self, who: &str) -> impl Future<Output = Option<Holds>> + Send;
    /// Whether this account has a way to change `moderator` (else VIP) at all.
    fn can_step(&self, moderator: bool) -> Result<(), String>;
    /// Gives (`on`) or takes `moderator` (else VIP).
    fn step(&self, who: &str, moderator: bool, on: bool) -> impl Future<Output = Result<(), Refused>> + Send;
    /// Marks (or unmarks) the echo of our own change, so it adds no line.
    fn mark(&self, action: RoleAction, who: &str, on: bool);
    /// A change went through: the role buttons hear of it.
    fn standing(&self, who: &str, moderator: bool, on: bool);
    /// The one chat line.
    fn notice(&self, line: &str);
}

/// 1.50 One mod or VIP change. V107-M8 Granting one to someone who holds the other takes
/// that away first (Twitch refuses to stack them) and leaves one chat line for both; a
/// grant Twitch refuses for that reason (stale badges) swaps too. `Err` is the line to
/// show; a swap shows its own line and returns `Ok`.
pub async fn set_role(ops: &impl RoleOps, who: &str, moderator: bool, on: bool) -> Result<(), String> {
    let clash = on && ops.holds(who).await.is_some_and(|h| h.clash(moderator));
    if !clash {
        match ops.step(who, moderator, on).await {
            Ok(()) => {
                ops.standing(who, moderator, on);
                return Ok(());
            }
            Err(r) if !r.clash => return Err(r.text),
            Err(_) => {}
        }
    }
    swap(ops, who, moderator).await
}

/// V107-M8 Takes the other role away, then grants `moderator` (else VIP).
async fn swap(ops: &impl RoleOps, who: &str, moderator: bool) -> Result<(), String> {
    let (gone, got) = (action(!moderator, false), action(moderator, true));
    let mark = |m: bool| {
        ops.mark(gone, who, m);
        ops.mark(got, who, m);
    };
    // What can be checked before anything goes: a token that may grant. Slots can't be
    // (Helix has no count), so a refused grant puts the old role back below.
    ops.can_step(moderator)?;
    mark(true);
    if let Err(r) = ops.step(who, !moderator, false).await {
        mark(false);
        return Err(r.text);
    }
    ops.standing(who, !moderator, false);
    if let Err(r) = ops.step(who, moderator, true).await {
        // Nobody should end up with neither: the old role goes back, its echo adds no line.
        ops.mark(action(!moderator, true), who, true);
        let restored = ops.step(who, !moderator, true).await.is_ok();
        for a in unmark_after_failed_grant(moderator, restored) {
            ops.mark(a, who, false);
        }
        if restored {
            ops.standing(who, !moderator, true);
        }
        // The one line for it all. Shown here, so the caller shows nothing more.
        ops.notice(&grant_failed(who, moderator, &r.text, r.slots, restored));
        return Ok(());
    }
    ops.standing(who, moderator, true);
    ops.notice(&swapped(who, moderator));
    Ok(())
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

/// V107-R14 The role change sequence against a scripted adapter.
#[cfg(test)]
mod sequence {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;

    /// Answers `step` from a script and writes down every call.
    struct Fake {
        holds: Option<Holds>,
        can: Result<(), String>,
        steps: Mutex<VecDeque<Result<(), Refused>>>,
        log: Mutex<Vec<String>>,
    }

    impl Fake {
        fn new(holds: Option<Holds>, steps: Vec<Result<(), Refused>>) -> Fake {
            Fake { holds, can: Ok(()), steps: Mutex::new(steps.into()), log: Mutex::default() }
        }

        fn note(&self, s: String) {
            self.log.lock().unwrap().push(s);
        }

        fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
    }

    fn role(moderator: bool) -> &'static str {
        if moderator { "mod" } else { "vip" }
    }

    impl RoleOps for Fake {
        fn holds(&self, _who: &str) -> impl Future<Output = Option<Holds>> + Send {
            std::future::ready(self.holds)
        }

        fn can_step(&self, _moderator: bool) -> Result<(), String> {
            self.can.clone()
        }

        fn step(&self, who: &str, moderator: bool, on: bool) -> impl Future<Output = Result<(), Refused>> + Send {
            self.note(format!("{} {} {who}", if on { "give" } else { "take" }, role(moderator)));
            let r = self.steps.lock().unwrap().pop_front().expect("a step nobody scripted");
            std::future::ready(r)
        }

        fn mark(&self, action: RoleAction, who: &str, on: bool) {
            self.note(format!("{} {} {who}", if on { "mark" } else { "unmark" }, action.as_str()));
        }

        fn standing(&self, who: &str, moderator: bool, on: bool) {
            self.note(format!("standing {who} {} {on}", role(moderator)));
        }

        fn notice(&self, line: &str) {
            self.note(format!("notice {line}"));
        }
    }

    /// Every future here is ready at once, so one poll runs it to the end.
    fn run(fake: &Fake, moderator: bool, on: bool) -> Result<(), String> {
        let mut f = std::pin::pin!(set_role(fake, "bob", moderator, on));
        match f.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
            std::task::Poll::Ready(r) => r,
            std::task::Poll::Pending => panic!("the fake never waits"),
        }
    }

    const VIP: Option<Holds> = Some(Holds { moderator: false, vip: true });
    const MOD: Option<Holds> = Some(Holds { moderator: true, vip: false });

    fn no(text: &str) -> Result<(), Refused> {
        Err(text.into())
    }

    #[test]
    fn a_plain_change_is_one_step() {
        let f = Fake::new(None, vec![Ok(())]);
        assert_eq!(run(&f, true, true), Ok(()));
        assert_eq!(f.log(), ["give mod bob", "standing bob mod true"]);
        let f = Fake::new(MOD, vec![no("You can’t do that here.")]);
        assert_eq!(run(&f, true, false), Err("You can’t do that here.".into()));
        assert_eq!(f.log(), ["take mod bob"], "a refusal changes nothing");
    }

    #[test]
    fn a_vip_made_a_mod_loses_the_vip_first() {
        let f = Fake::new(VIP, vec![Ok(()), Ok(())]);
        assert_eq!(run(&f, true, true), Ok(()));
        assert_eq!(f.log(), [
            "mark unvip bob", "mark mod bob",
            "take vip bob", "standing bob vip false",
            "give mod bob", "standing bob mod true",
            "notice Removed bob as VIP and made them a mod.",
        ]);
    }

    #[test]
    fn stale_badges_swap_when_twitch_says_clash() {
        let clash = Err(Refused { text: "x".into(), clash: true, slots: false });
        let f = Fake::new(None, vec![clash, Ok(()), Ok(())]);
        assert_eq!(run(&f, false, true), Ok(()));
        assert_eq!(f.log()[0], "give vip bob");
        assert_eq!(f.log().last().unwrap(), "notice Removed bob as mod and made them a VIP.");
    }

    #[test]
    fn no_token_takes_nothing_away() {
        let mut f = Fake::new(VIP, vec![]);
        f.can = Err(LEAD_NEEDS_GQL.into());
        assert_eq!(run(&f, true, true), Err(LEAD_NEEDS_GQL.into()));
        assert!(f.log().is_empty());
    }

    #[test]
    fn remove_fails() {
        let f = Fake::new(VIP, vec![no("Twitch is down.")]);
        assert_eq!(run(&f, true, true), Err("Twitch is down.".into()));
        assert_eq!(f.log(), ["mark unvip bob", "mark mod bob", "take vip bob", "unmark unvip bob", "unmark mod bob"]);
    }

    #[test]
    fn grant_fails_and_the_restore_succeeds() {
        let f = Fake::new(VIP, vec![Ok(()), no("You can’t do that here."), Ok(())]);
        assert_eq!(run(&f, true, true), Ok(()));
        assert_eq!(f.log(), [
            "mark unvip bob", "mark mod bob",
            "take vip bob", "standing bob vip false",
            "give mod bob", "mark vip bob", "give vip bob", "unmark mod bob",
            "standing bob vip true",
            "notice Couldn’t make bob a mod: You can’t do that here. Kept them VIP.",
        ]);
    }

    #[test]
    fn grant_fails_and_the_restore_fails() {
        let f = Fake::new(VIP, vec![Ok(()), no("You can’t do that here."), no("Twitch is down.")]);
        assert_eq!(run(&f, true, true), Ok(()));
        assert_eq!(f.log(), [
            "mark unvip bob", "mark mod bob",
            "take vip bob", "standing bob vip false",
            "give mod bob", "mark vip bob", "give vip bob", "unmark mod bob", "unmark vip bob",
            "notice Removed bob as VIP, but couldn’t make them a mod: You can’t do that here.",
        ]);
    }

    #[test]
    fn no_vip_slots_keeps_them_mod() {
        let full = Err(Refused { text: "No VIP slots left.".into(), clash: false, slots: true });
        let f = Fake::new(MOD, vec![Ok(()), full, Ok(())]);
        assert_eq!(run(&f, false, true), Ok(()));
        let log = f.log();
        assert_eq!(&log[..3], ["mark unmod bob", "mark vip bob", "take mod bob"]);
        assert_eq!(log.last().unwrap(), "notice Couldn’t make bob VIP: no VIP slots left. Kept them mod.");
        assert!(log.contains(&"standing bob mod true".to_owned()), "back to mod");
    }
}
