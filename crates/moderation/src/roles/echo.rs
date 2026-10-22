//! V107-M8 The internal seam behind the app's adapter: the mod feeds echo every mod and VIP
//! change (and a lead mod grant, as "mod"), and [`super::change`] already says it in one
//! line, so each of our steps marks its echo as ours ([`Echoed`]). The role buttons hear of
//! every step that went through.

use cyex_chat::model::ModRole;

use super::{Path, Refusal, Role, Twitch};

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

    /// The feed action a step echoes as. Twitch echoes a lead mod grant as "mod" (seen live
    /// 2026/10/20); the rest have no feed action.
    fn of_step(role: Role, on: bool) -> Option<RoleAction> {
        match role {
            Role::Mod => Some(RoleAction::of(true, on)),
            Role::Vip => Some(RoleAction::of(false, on)),
            Role::LeadMod if on => Some(RoleAction::Mod),
            _ => None,
        }
    }
}

/// What someone holds, from the badge sets Twitch shows for them in a channel.
pub fn holds_of<'a>(sets: impl IntoIterator<Item = &'a str>) -> Vec<Role> {
    let mut out = Vec::new();
    for s in sets {
        let add: &[Role] = match s {
            "lead_moderator" => &[Role::LeadMod, Role::Mod],
            "moderator" => &[Role::Mod],
            "vip" => &[Role::Vip],
            _ => &[],
        };
        for r in add {
            if !out.contains(r) {
                out.push(*r);
            }
        }
    }
    out.sort();
    out
}

/// The app's chat side, for one account in one channel.
pub trait Chat: Sync {
    /// Marks (or unmarks) the echo of our own step, so it adds no line.
    fn mark(&self, action: RoleAction, who: &str, on: bool);
    /// A step went through: the role buttons hear of it (`moderator`, else VIP).
    fn standing(&self, who: &str, moderator: bool, on: bool);
}

/// A [`Twitch`] port whose steps mark their own echoes: marked before the step (the echo
/// can beat Twitch's answer), unmarked when it failed. A lead mod grant Twitch calls
/// `ALREADY_ASSIGNED` still echoes, so it stays marked.
pub struct Echoed<'a, T, C> {
    pub twitch: &'a T,
    pub chat: &'a C,
}

impl<T: Twitch, C: Chat> Twitch for Echoed<'_, T, C> {
    fn by(&self) -> ModRole {
        self.twitch.by()
    }

    fn path(&self, role: Role) -> Result<Path, &'static str> {
        self.twitch.path(role)
    }

    async fn assign(&self, who: &str, role: Role, on: bool) -> Result<(), Refusal> {
        let echo = RoleAction::of_step(role, on);
        if let Some(a) = echo {
            self.chat.mark(a, who, true);
        }
        let res = self.twitch.assign(who, role, on).await;
        let echoes = match &res {
            Ok(()) => true,
            Err(Refusal::Code(c)) => role == Role::LeadMod && c.eq_ignore_ascii_case("ALREADY_ASSIGNED"),
            Err(Refusal::Line(_)) => false,
        };
        if let Some(a) = echo.filter(|_| !echoes) {
            self.chat.mark(a, who, false);
        }
        // V107-N12d "Isn't a mod" says what they hold too.
        let settled = match &res {
            Ok(()) => true,
            Err(Refusal::Code(c)) => super::lines::holds_already(c, on),
            Err(Refusal::Line(_)) => false,
        };
        if settled {
            match role {
                Role::Mod | Role::Vip => self.chat.standing(who, role == Role::Mod, on),
                Role::LeadMod if on => self.chat.standing(who, true, true),
                _ => {}
            }
        }
        res
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;

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
    fn v107_m8_holds_from_badges() {
        assert!(holds_of(["subscriber"]).is_empty());
        assert_eq!(holds_of(["lead_moderator"]), [Role::LeadMod, Role::Mod]);
        assert_eq!(holds_of(["vip", "subscriber", "vip"]), [Role::Vip]);
        assert_eq!(holds_of(["moderator", "lead_moderator"]), [Role::LeadMod, Role::Mod]);
    }

    /// Scripted Twitch: answers `assign` from a script and writes every call down, in one
    /// log with the chat side's calls. Shared with `change`'s tests.
    pub struct Fake {
        pub by: ModRole,
        pub path: Result<Path, &'static str>,
        pub answers: Mutex<VecDeque<Result<(), Refusal>>>,
        pub log: Mutex<Vec<String>>,
    }

    impl Fake {
        pub fn new(answers: Vec<Result<(), Refusal>>) -> Fake {
            Fake { by: ModRole::Broadcaster, path: Ok(Path::Gql), answers: Mutex::new(answers.into()), log: Mutex::default() }
        }

        pub fn note(&self, s: String) {
            self.log.lock().unwrap().push(s);
        }

        pub fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
    }

    pub fn step(role: Role, on: bool) -> String {
        format!("{} {}", if on { "give" } else { "take" }, role.name())
    }

    impl Twitch for Fake {
        fn by(&self) -> ModRole {
            self.by
        }

        fn path(&self, _role: Role) -> Result<Path, &'static str> {
            self.path
        }

        fn assign(&self, _who: &str, role: Role, on: bool) -> impl Future<Output = Result<(), Refusal>> + Send {
            self.note(step(role, on));
            let r = self.answers.lock().unwrap().pop_front().expect("a step nobody scripted");
            std::future::ready(r)
        }
    }

    impl Chat for Fake {
        fn mark(&self, action: RoleAction, who: &str, on: bool) {
            self.note(format!("{} {} {who}", if on { "mark" } else { "unmark" }, action.as_str()));
        }

        fn standing(&self, who: &str, moderator: bool, on: bool) {
            self.note(format!("standing {who} {} {on}", if moderator { "mod" } else { "vip" }));
        }
    }

    /// Every future here is ready at once, so one poll runs it to the end.
    pub fn now<F: Future>(f: F) -> F::Output {
        let mut f = std::pin::pin!(f);
        match f.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
            std::task::Poll::Ready(r) => r,
            std::task::Poll::Pending => panic!("the fake never waits"),
        }
    }

    pub fn code(c: &str) -> Result<(), Refusal> {
        Err(Refusal::Code(c.into()))
    }

    #[test]
    fn v107_m8_a_step_marks_its_echo_and_tells_the_buttons() {
        let f = Fake::new(vec![Ok(()), code("FORBIDDEN"), Ok(())]);
        let e = Echoed { twitch: &f, chat: &f };
        assert_eq!(now(e.assign("bob", Role::Vip, false)), Ok(()));
        assert!(now(e.assign("bob", Role::Mod, true)).is_err());
        assert_eq!(now(e.assign("bob", Role::Editor, true)), Ok(()));
        assert_eq!(f.log(), [
            "mark unvip bob", "take VIP", "standing bob vip false",
            "mark mod bob", "give mod", "unmark mod bob",
            "give editor",
        ]);
    }

    #[test]
    fn v107_r28_a_lead_mod_grant_echoes_as_mod() {
        let f = Fake::new(vec![Ok(()), code("ALREADY_ASSIGNED"), code("PERMISSION_DENIED"), Ok(())]);
        let e = Echoed { twitch: &f, chat: &f };
        let _ = now(e.assign("bob", Role::LeadMod, true));
        let _ = now(e.assign("bob", Role::LeadMod, true));
        let _ = now(e.assign("bob", Role::LeadMod, true));
        let _ = now(e.assign("bob", Role::LeadMod, false));
        assert_eq!(f.log(), [
            "mark mod bob", "give lead mod", "standing bob mod true",
            "mark mod bob", "give lead mod", "standing bob mod true",
            "mark mod bob", "give lead mod", "unmark mod bob",
            "take lead mod",
        ]);
    }

    /// V107-N12d A stale badge offered Unmod: "isn't a mod" tells the buttons, so the
    /// next menu offers Mod.
    #[test]
    fn v107_n12d_already_so_tells_the_buttons() {
        let f = Fake::new(vec![code("TARGET_NOT_MOD"), code("GRANTEE_ALREADY_VIP"), code("TARGET_NOT_MOD")]);
        let e = Echoed { twitch: &f, chat: &f };
        assert!(now(e.assign("bob", Role::Mod, false)).is_err());
        assert!(now(e.assign("bob", Role::Vip, true)).is_err());
        assert!(now(e.assign("bob", Role::Mod, true)).is_err(), "not this code when giving");
        assert_eq!(f.log(), [
            "mark unmod bob", "take mod", "unmark unmod bob", "standing bob mod false",
            "mark vip bob", "give VIP", "unmark vip bob", "standing bob vip true",
            "mark mod bob", "give mod", "unmark mod bob",
        ]);
    }
}
