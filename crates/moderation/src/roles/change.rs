//! V107-R42 One role change, start to end: the plan (what goes first), the checks before
//! anything is sent, the steps, the rollback and the one line.

use cyex_chat::model::ModRole;

use super::lines;
use super::{Change, Outcome, Refusal, Role, Twitch, VipSlots, Wanted};

/// One asked-for change and the step that has to go first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Unit {
    first: Option<Change>,
    main: Change,
}

/// V107-M8 The role Twitch won't stack with `give`: a VIP can't be a mod or lead mod.
fn other(give: Role) -> Option<Role> {
    match give {
        Role::Mod | Role::LeadMod => Some(Role::Vip),
        Role::Vip => Some(Role::Mod),
        _ => None,
    }
}

/// V107-M8 Whether Twitch refused a grant because they hold the other role (the holds were
/// stale). GQL's codes for it aren't all seen yet, so they're read loosely: a code that
/// names the other role and isn't an "already"; Helix's 422 comes as one (`helix_code`).
fn clashed(c: Change, r: &Refusal) -> bool {
    let Refusal::Code(code) = r else { return false };
    let code = code.to_ascii_uppercase();
    if !c.on || code.contains("ALREADY") || other(c.role).is_none() {
        return false;
    }
    code == "CANNOT_BE_VIP_AND_MOD"
        || match c.role {
            Role::Vip => code.contains("MOD"),
            _ => code.contains("VIP") && !code.starts_with("MAX_VIPS") && !code.contains("ACHIEVEMENT"),
        }
}

/// The units in the order they run: takes first (they free slots and clear clashes), then
/// grants in Twitch's role order. Twitch makes a lead mod a mod (seen live 2026/10/21), so
/// giving both sends lead mod only. A role in both lists is given.
fn plan(who: &str, holds: &[Role], w: &Wanted) -> Result<Vec<Unit>, String> {
    let mut give: Vec<Role> = Role::ALL.into_iter().filter(|r| w.give.contains(r)).collect();
    let take: Vec<Role> = Role::ALL.into_iter().filter(|r| w.take.contains(r) && !give.contains(r)).collect();
    if give.contains(&Role::LeadMod) {
        give.retain(|r| *r != Role::Mod);
    }
    if give.contains(&Role::Vip) && give.iter().any(|r| other(*r) == Some(Role::Vip)) {
        return Err(format!("{who} can’t be a VIP and a mod."));
    }
    let mut out: Vec<Unit> = take.iter().map(|r| Unit { first: None, main: Change::new(*r, false) }).collect();
    for r in give {
        let first = other(r).filter(|o| holds.contains(o) && !take.contains(o)).map(|o| Change::new(o, false));
        out.push(Unit { first, main: Change::new(r, true) });
    }
    Ok(out)
}

/// A change while it runs: what stuck so far and what the slots taught.
struct Run<'a, T> {
    t: &'a T,
    who: &'a str,
    by: ModRole,
    slots: VipSlots,
    done: Vec<Change>,
}

impl<T: Twitch> Run<'_, T> {
    async fn step(&mut self, c: Change) -> Result<(), Refusal> {
        let res = self.t.assign(self.who, c.role, c.on).await;
        match &res {
            Ok(()) => {
                // An undo cancels the step it undoes.
                match self.done.iter().rposition(|d| *d == c.undo()) {
                    Some(i) => {
                        self.done.remove(i);
                    }
                    None => self.done.push(c),
                }
                if c.role == Role::Vip {
                    self.slots = self.slots.moved(c.on);
                }
            }
            Err(Refusal::Code(code)) if c.role == Role::Vip && c.on => self.slots = self.slots.learn(code),
            Err(_) => {}
        }
        res
    }

    fn line(&self, r: &Refusal, c: Change) -> String {
        lines::line(r, self.who, c, self.by)
    }

    /// One unit; the line either way.
    async fn unit(&mut self, u: Unit) -> Result<String, String> {
        let who = self.who;
        let mut first = u.first;
        if let Some(f) = first {
            self.step(f).await.map_err(|r| self.line(&r, f))?;
        }
        let mut res = self.step(u.main).await;
        if let (None, Err(r), Some(o)) = (first, &res, other(u.main.role))
            && clashed(u.main, r)
            && self.t.path(o).is_ok()
        {
            // Stale holds: the other role goes first after all, then the grant once more.
            let f = Change::new(o, false);
            self.step(f).await.map_err(|r| self.line(&r, f))?;
            first = Some(f);
            res = self.step(u.main).await;
        }
        match (res, first) {
            (Ok(()), None) => Ok(lines::done(who, u.main)),
            (Ok(()), Some(f)) => Ok(lines::swapped(who, f.role, u.main.role)),
            (Err(r), None) => Err(self.line(&r, u.main)),
            (Err(r), Some(f)) => {
                // Nobody should end up with neither: the old role goes back.
                let why = lines::clause(&r, who, u.main, self.by);
                if self.step(f.undo()).await.is_ok() {
                    Err(lines::kept(who, f.role, u.main.role, &why))
                } else {
                    Err(lines::half_done(who, f.role, u.main.role, &why))
                }
            }
        }
    }
}

/// Runs one change for `who` (a login), given what they hold (the roles manager's row, or
/// their badges; may be stale) and the channel's VIP slots. Nothing is sent when a step has
/// no way to go ([`Twitch::path`]) or a VIP grant has no slot. Units run in order and stop
/// at the first that fails; a failed grant puts back what went first for it.
pub async fn change(t: &impl Twitch, who: &str, holds: &[Role], slots: VipSlots, wanted: &Wanted) -> Outcome {
    let asked: Vec<Role> = Role::ALL.into_iter().filter(|r| wanted.give.contains(r) || wanted.take.contains(r)).collect();
    let refuse = |line: String, slots: VipSlots| Outcome { done: Vec::new(), line, slots, failed: asked.clone() };
    let units = match plan(who, holds, wanted) {
        Ok(u) => u,
        Err(line) => return refuse(line, slots),
    };
    if units.is_empty() {
        return Outcome { done: Vec::new(), line: "Nothing to change.".into(), slots, failed: Vec::new() };
    }
    for c in units.iter().flat_map(|u| u.first.into_iter().chain([u.main])) {
        if let Err(line) = t.path(c.role) {
            return refuse(line.into(), slots);
        }
    }
    let vip_grant = units.iter().any(|u| u.main == Change::new(Role::Vip, true));
    if vip_grant && !holds.contains(&Role::Vip) {
        // No slot to fill: nothing is taken away first.
        if let Some(why) = slots.why {
            return refuse(why.into(), slots);
        }
    }
    let mut run = Run { t, who, by: t.by(), slots, done: Vec::new() };
    let mut said: Vec<String> = Vec::new();
    let mut went: Vec<Change> = Vec::new();
    for (i, u) in units.iter().enumerate() {
        match run.unit(*u).await {
            Ok(line) => {
                said.push(line);
                went.push(u.main);
            }
            Err(line) => {
                let line = match went.len() {
                    0 => line,
                    1 => format!("{} {line}", said[0]),
                    _ => format!("{} {line}", lines::summary(who, &went)),
                };
                let failed = units[i..].iter().map(|u| u.main.role).collect();
                return Outcome { done: run.done, line, slots: run.slots, failed };
            }
        }
    }
    let line = if said.len() == 1 { said.remove(0) } else { lines::summary(who, &went) };
    Outcome { done: run.done, line, slots: run.slots, failed: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::super::echo::tests::{Fake, code, now, step};
    use super::super::{Echoed, FULL, LOCKED, VipState};
    use super::*;
    use Role::*;

    fn run(f: &Fake, holds: &[Role], slots: VipSlots, w: Wanted) -> Outcome {
        now(change(f, "bob", holds, slots, &w))
    }

    fn give(r: &[Role]) -> Wanted {
        Wanted { give: r.to_vec(), take: vec![] }
    }

    fn line(s: &str) -> Result<(), Refusal> {
        Err(Refusal::Line(s.into()))
    }

    fn unknown() -> VipSlots {
        VipSlots::unknown()
    }

    #[test]
    fn a_plain_change_is_one_step() {
        let f = Fake::new(vec![Ok(())]);
        let o = run(&f, &[], unknown(), give(&[Mod]));
        assert_eq!((o.line.as_str(), o.ok()), ("Made bob a mod.", true));
        assert_eq!(o.done, [Change::new(Mod, true)]);
        assert_eq!(f.log(), [step(Mod, true)]);
        let f = Fake::new(vec![code("FORBIDDEN")]);
        let o = run(&f, &[Mod], unknown(), Wanted::one(Mod, false));
        assert_eq!((o.line.as_str(), o.failed.as_slice()), ("You can’t do that here.", &[Mod][..]));
        assert!(o.done.is_empty(), "a refusal changes nothing");
    }

    #[test]
    fn v107_m8_a_vip_made_a_mod_loses_the_vip_first() {
        let f = Fake::new(vec![Ok(()), Ok(())]);
        let o = now(change(&Echoed { twitch: &f, chat: &f }, "bob", &[Vip], unknown(), &give(&[Mod])));
        assert_eq!(o.line, "Removed bob as VIP and made them a mod.");
        assert_eq!(o.done, [Change::new(Vip, false), Change::new(Mod, true)]);
        assert_eq!(f.log(), [
            "mark unvip bob", "take VIP", "standing bob vip false",
            "mark mod bob", "give mod", "standing bob mod true",
        ]);
    }

    #[test]
    fn v107_m8_stale_holds_swap_when_twitch_says_clash() {
        let f = Fake::new(vec![code("GRANTEE_IS_MOD"), Ok(()), Ok(())]);
        let o = run(&f, &[], unknown(), give(&[Vip]));
        assert_eq!(f.log(), [step(Vip, true), step(Mod, false), step(Vip, true)]);
        assert_eq!(o.line, "Removed bob as mod and made them a VIP.");
        // Helix's 422 for a mod grant, read as its code.
        let f = Fake::new(vec![code("TARGET_IS_VIP"), Ok(()), Ok(())]);
        assert!(run(&f, &[], unknown(), give(&[Mod])).ok());
        // "Already" is no clash.
        let f = Fake::new(vec![code("TARGET_ALREADY_MOD")]);
        assert_eq!(run(&f, &[], unknown(), give(&[Mod])).line, "bob is already a mod.");
        assert_eq!(f.log().len(), 1);
    }

    #[test]
    fn no_way_to_send_takes_nothing_away() {
        let mut f = Fake::new(vec![]);
        f.path = Err(super::super::LEAD_NEEDS_GQL);
        let o = run(&f, &[Vip], unknown(), give(&[Mod]));
        assert_eq!(o.line, super::super::LEAD_NEEDS_GQL);
        assert!(f.log().is_empty());
        let f = Fake::new(vec![]);
        assert_eq!(run(&f, &[], unknown(), give(&[Vip, Mod])).line, "bob can’t be a VIP and a mod.");
        assert_eq!(run(&f, &[], unknown(), Wanted::default()).line, "Nothing to change.");
        assert!(f.log().is_empty());
    }

    #[test]
    fn remove_fails() {
        let f = Fake::new(vec![line("Twitch is down.")]);
        let o = run(&f, &[Vip], unknown(), give(&[Mod]));
        assert_eq!(o.line, "Twitch is down.");
        assert!(o.done.is_empty());
        assert_eq!(f.log(), [step(Vip, false)]);
    }

    #[test]
    fn v107_m8_grant_fails_and_the_restore_succeeds() {
        let f = Fake::new(vec![Ok(()), code("FORBIDDEN"), Ok(())]);
        let o = now(change(&Echoed { twitch: &f, chat: &f }, "bob", &[Vip], unknown(), &give(&[Mod])));
        assert_eq!(o.line, "Couldn’t make bob a mod: You can’t do that here. Kept them VIP.");
        assert!(o.done.is_empty(), "back where it started");
        assert_eq!(o.failed, [Mod]);
        assert_eq!(f.log(), [
            "mark unvip bob", "take VIP", "standing bob vip false",
            "mark mod bob", "give mod", "unmark mod bob",
            "mark vip bob", "give VIP", "standing bob vip true",
        ]);
    }

    #[test]
    fn v107_m8_grant_fails_and_the_restore_fails() {
        let f = Fake::new(vec![Ok(()), code("FORBIDDEN"), line("Twitch is down.")]);
        let o = now(change(&Echoed { twitch: &f, chat: &f }, "bob", &[Vip], unknown(), &give(&[Mod])));
        assert_eq!(o.line, "Removed bob as VIP, but couldn’t make them a mod: You can’t do that here.");
        assert_eq!(o.done, [Change::new(Vip, false)]);
        let log = f.log();
        assert_eq!(&log[log.len() - 3..], ["mark vip bob", "give VIP", "unmark vip bob"], "the removal's mark stays");
    }

    #[test]
    fn v107_r42_no_vip_slots_are_learned_and_keep_them_mod() {
        let f = Fake::new(vec![Ok(()), code("MAX_VIPS_REACHED"), Ok(())]);
        let o = run(&f, &[Mod], VipSlots::new(4, None), give(&[Vip]));
        assert_eq!(o.line, "Couldn’t make bob a VIP: no VIP slots left. Kept them mod.");
        assert_eq!(o.slots.state, VipState::Full, "learned from the code");
        assert_eq!(f.log(), [step(Mod, false), step(Vip, true), step(Mod, true)]);
        let f = Fake::new(vec![code("VIP_ACHIEVEMENT_INCOMPLETE")]);
        assert_eq!(run(&f, &[], unknown(), give(&[Vip])).slots.why, Some(LOCKED));
    }

    #[test]
    fn v107_r42_known_slots_refuse_before_sending() {
        let f = Fake::new(vec![]);
        let o = run(&f, &[Mod], VipSlots::new(0, Some(0)), give(&[Vip]));
        assert_eq!(o.line, LOCKED);
        let o = run(&f, &[], VipSlots::new(10, Some(10)), give(&[Vip]));
        assert_eq!((o.line.as_str(), o.failed.as_slice()), (FULL, &[Vip][..]));
        assert!(f.log().is_empty(), "nothing sent, nothing taken away");
        // Taking a VIP needs no slot.
        let f = Fake::new(vec![Ok(())]);
        let o = run(&f, &[Vip], VipSlots::new(10, Some(10)), Wanted::one(Vip, false));
        assert_eq!((o.line.as_str(), o.slots.used, o.slots.state), ("bob is no longer a VIP.", 9, VipState::Open));
    }

    #[test]
    fn v107_r42_vip_goes_before_lead_mod() {
        let f = Fake::new(vec![Ok(()), Ok(())]);
        let o = run(&f, &[Vip, Editor], unknown(), give(&[LeadMod, Mod]));
        assert_eq!(f.log(), [step(Vip, false), step(LeadMod, true)], "Twitch makes a lead mod a mod");
        assert_eq!(o.line, "Removed bob as VIP and made them a lead mod.");
        let f = Fake::new(vec![Ok(())]);
        run(&f, &[Mod], unknown(), give(&[LeadMod]));
        assert_eq!(f.log(), [step(LeadMod, true)], "mod stays");
    }

    #[test]
    fn v107_r42_a_failed_lead_mod_puts_the_vip_back_last() {
        let f = Fake::new(vec![Ok(()), code("ROLE_COUNT_LIMIT_REACHED"), Ok(())]);
        let o = run(&f, &[Vip], VipSlots::new(3, Some(5)), give(&[LeadMod]));
        assert_eq!(f.log(), [step(Vip, false), step(LeadMod, true), step(Vip, true)]);
        assert_eq!(o.line, "Couldn’t make bob a lead mod: No lead mod slots left. Kept them VIP.");
        assert_eq!(o.slots.used, 3, "the VIP went and came back");
    }

    #[test]
    fn v107_r42_several_roles_stop_at_the_first_refusal() {
        let f = Fake::new(vec![Ok(()), code("MAX_ARTISTS_REACHED")]);
        let o = run(&f, &[], unknown(), give(&[Agent, Artist, Editor]));
        assert_eq!(f.log(), [step(Editor, true), step(Artist, true)], "Twitch's order; agent never sent");
        assert_eq!(o.line, "Made bob an editor. No artist slots left.");
        assert_eq!(o.done, [Change::new(Editor, true)]);
        assert_eq!(o.failed, [Artist, Agent]);
        let f = Fake::new(vec![Ok(()), Ok(()), Ok(())]);
        let o = run(&f, &[], unknown(), Wanted { give: vec![Editor, Agent], take: vec![Artist] });
        assert_eq!(f.log(), [step(Artist, false), step(Editor, true), step(Agent, true)], "takes first");
        assert_eq!(o.line, "Changed bob’s roles.");
        let f = Fake::new(vec![Ok(()), Ok(())]);
        assert_eq!(run(&f, &[], unknown(), give(&[Editor, Artist])).line, "Made bob an editor and an artist.");
    }

    #[test]
    fn v107_r42_codes_read_by_the_one_table() {
        for (c, want) in [
            ("GRANTEE_CHAT_BANNED", "bob is banned here. Unban them first."),
            ("NOT_DONE", "Twitch didn’t do it. Try again."),
            ("NEW_THING", "Twitch refused it (NEW_THING)."),
        ] {
            let f = Fake::new(vec![code(c)]);
            assert_eq!(run(&f, &[], unknown(), give(&[Editor])).line, want);
        }
        let mut f = Fake::new(vec![code("FORBIDDEN")]);
        f.by = ModRole::Moderator;
        assert_eq!(run(&f, &[], unknown(), give(&[Vip])).line, crate::command::Need::Lead.refusal());
    }
}
