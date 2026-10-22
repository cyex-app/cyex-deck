//! V107-Mw Your own roles from chat: `/unmod` on yourself steps down as mod the way My
//! roles does (V107-Mj: lead mod first, then mod), `/unleadmod` on yourself steps down
//! from lead mod only. Every other change on yourself stays refused (V107-N2f: a mod's
//! `/vip <self>` took their mod role and then couldn't give anything).

use cyex_chat::model::ModRole;

use super::lines;
use super::{Change, Outcome, Refusal, Role, VipSlots, Wanted};

/// The line for any other change on yourself.
pub const SELF: &str = "That’s you.";

/// Leaves one of your own roles in the channel (the dashboard's step-down mutations).
pub trait Leave: Sync {
    fn leave(&self, role: Role) -> impl Future<Output = Result<(), Refusal>> + Send;
}

/// What a change on yourself may do: `Mod` steps down as mod (lead mod too, when held),
/// `LeadMod` from lead mod only. `None`: refused ([`SELF`]).
pub fn own_step(w: &Wanted) -> Option<Role> {
    if !w.give.is_empty() {
        return None;
    }
    let other = w.take.iter().any(|r| !matches!(r, Role::Mod | Role::LeadMod));
    match (other, w.take.contains(&Role::Mod), w.take.contains(&Role::LeadMod)) {
        (false, true, _) => Some(Role::Mod),
        (false, false, true) => Some(Role::LeadMod),
        _ => None,
    }
}

/// Why a step down didn't go, as a clause.
fn why(r: &Refusal, role: Role, by: ModRole) -> String {
    let w = match r {
        Refusal::Line(l) => l.clone(),
        Refusal::Code(code) => match code.to_ascii_uppercase().as_str() {
            "TARGET_NOT_MOD" | "NOT_ASSIGNED" | "REVOKEE_DOES_NOT_HAVE_ROLE" => format!("You aren’t {} here.", lines::a(role)),
            "PERMISSION_DENIED" | "FORBIDDEN" | "UNAUTHORIZED" | "NO_PERMISSIONS" => "Twitch didn’t allow that.".into(),
            _ => lines::refused(code, "you", Change::new(role, false), by),
        },
    };
    w.trim_end_matches('.').to_owned()
}

fn outcome(done: Vec<Change>, line: String, failed: Vec<Role>) -> Outcome {
    Outcome { done, line, slots: VipSlots::unknown(), failed }
}

/// Steps down as `role` (from [`own_step`]). For mod with `lead` (you hold lead mod there,
/// as far as known) lead mod goes first, since `unmodUser` leaves a lead mod one; if that's
/// refused nothing else is sent. Twitch saying lead mod wasn't held goes on to mod.
pub async fn step_down(t: &impl Leave, role: Role, lead: bool, by: ModRole) -> Outcome {
    let gone = |r: Role| Change::new(r, false);
    if role == Role::LeadMod {
        return match t.leave(Role::LeadMod).await {
            Ok(()) => outcome(vec![gone(Role::LeadMod)], "You stepped down as lead mod.".into(), vec![]),
            Err(r) => outcome(vec![], format!("{}.", why(&r, Role::LeadMod, by)), vec![Role::LeadMod]),
        };
    }
    let mut done = Vec::new();
    if lead {
        match t.leave(Role::LeadMod).await {
            Ok(()) => done.push(gone(Role::LeadMod)),
            Err(Refusal::Code(c)) if lines::holds_already(&c, false) => {}
            Err(r) => {
                let line = format!("Couldn’t remove your lead mod role: {}. You’re still a mod.", why(&r, Role::LeadMod, by));
                return outcome(done, line, vec![Role::Mod]);
            }
        }
    }
    match t.leave(Role::Mod).await {
        Ok(()) => {
            let line = if done.is_empty() { "You stepped down as mod." } else { "You stepped down as mod and lead mod." };
            done.push(gone(Role::Mod));
            outcome(done, line.into(), vec![])
        }
        Err(r) => {
            let w = why(&r, Role::Mod, by);
            let line = if done.is_empty() { format!("{w}.") } else { format!("You’re no longer a lead mod. {w}.") };
            outcome(done, line, vec![Role::Mod])
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::super::echo::tests::{code, now};
    use super::*;
    use Role::*;

    struct Fake {
        answers: Mutex<VecDeque<Result<(), Refusal>>>,
        log: Mutex<Vec<Role>>,
    }

    impl Fake {
        fn new(a: Vec<Result<(), Refusal>>) -> Fake {
            Fake { answers: Mutex::new(a.into()), log: Mutex::default() }
        }
        fn log(&self) -> Vec<Role> {
            self.log.lock().unwrap().clone()
        }
    }

    impl Leave for Fake {
        fn leave(&self, role: Role) -> impl Future<Output = Result<(), Refusal>> + Send {
            self.log.lock().unwrap().push(role);
            std::future::ready(self.answers.lock().unwrap().pop_front().expect("unscripted step"))
        }
    }

    fn run(f: &Fake, role: Role, lead: bool) -> Outcome {
        now(step_down(f, role, lead, ModRole::LeadModerator))
    }

    #[test]
    fn v107_mw_what_a_change_on_yourself_may_do() {
        assert_eq!(own_step(&Wanted::one(Mod, false)), Some(Mod));
        assert_eq!(own_step(&Wanted { give: vec![], take: vec![LeadMod, Mod] }), Some(Mod));
        assert_eq!(own_step(&Wanted::one(LeadMod, false)), Some(LeadMod));
        assert_eq!(own_step(&Wanted::one(Vip, true)), None, "V107-N2f /vip self stays refused");
        assert_eq!(own_step(&Wanted::one(Mod, true)), None);
        assert_eq!(own_step(&Wanted::one(Vip, false)), None);
        assert_eq!(own_step(&Wanted { give: vec![], take: vec![Mod, Editor] }), None);
        assert_eq!(own_step(&Wanted::default()), None);
    }

    #[test]
    fn v107_mw_unmod_yourself_takes_lead_mod_first() {
        let f = Fake::new(vec![Ok(()), Ok(())]);
        let o = run(&f, Mod, true);
        assert_eq!((o.line.as_str(), o.ok()), ("You stepped down as mod and lead mod.", true));
        assert_eq!(o.done, [Change::new(LeadMod, false), Change::new(Mod, false)]);
        assert_eq!(f.log(), [LeadMod, Mod]);
        let f = Fake::new(vec![Ok(())]);
        assert_eq!(run(&f, Mod, false).line, "You stepped down as mod.");
        assert_eq!(f.log(), [Mod]);
        // Stale: not a lead mod after all.
        let f = Fake::new(vec![code("NOT_ASSIGNED"), Ok(())]);
        assert_eq!(run(&f, Mod, true).line, "You stepped down as mod.");
    }

    #[test]
    fn v107_mw_refusals_stop_and_say_so() {
        let f = Fake::new(vec![code("FORBIDDEN")]);
        let o = run(&f, Mod, true);
        assert_eq!(o.line, "Couldn’t remove your lead mod role: Twitch didn’t allow that. You’re still a mod.");
        assert_eq!((o.done.len(), o.failed.as_slice()), (0, &[Mod][..]));
        assert_eq!(f.log(), [LeadMod], "mod never sent");
        let f = Fake::new(vec![Ok(()), Err(Refusal::Line("Twitch is down.".into()))]);
        let o = run(&f, Mod, true);
        assert_eq!(o.line, "You’re no longer a lead mod. Twitch is down.");
        assert_eq!(o.done, [Change::new(LeadMod, false)]);
        let f = Fake::new(vec![code("TARGET_NOT_MOD")]);
        assert_eq!(run(&f, Mod, false).line, "You aren’t a mod here.");
    }

    #[test]
    fn v107_mw_unleadmod_yourself_keeps_mod() {
        let f = Fake::new(vec![Ok(())]);
        let o = run(&f, LeadMod, true);
        assert_eq!((o.line.as_str(), o.done.as_slice()), ("You stepped down as lead mod.", &[Change::new(LeadMod, false)][..]));
        assert_eq!(f.log(), [LeadMod]);
        let f = Fake::new(vec![code("NOT_ASSIGNED")]);
        let o = run(&f, LeadMod, false);
        assert_eq!((o.line.as_str(), o.failed.as_slice()), ("You aren’t a lead mod here.", &[LeadMod][..]));
    }
}
