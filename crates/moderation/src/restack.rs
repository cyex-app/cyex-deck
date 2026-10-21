//! V107-M7 Timeouts stack so the last one counts: what to do when Twitch refuses a ban
//! or timeout because one is already in place. Pure, the app runs the plan.
//!
//! Helix (docs, Ban User): a timeout over a timeout replaces it, a ban over a timeout
//! works, a timeout over a ban is refused ("is already banned"); 409 means another change
//! to that user is in flight, retry. GQL replaces a timeout itself too (seen 2026/10/19:
//! 60 s, 30 s, 600 s in a row, each end replaced the last); its `TARGET_ALREADY_BANNED`
//! doesn't say what is in place, so the app looks the state up to know which.

/// How Twitch said no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal<'a> {
    Helix { status: u16, message: &'a str },
    Gql(&'a str),
}

/// What the target has right now, from Twitch's ban status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    Nothing,
    Timeout,
    Ban,
}

/// The next step after a refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    /// A real refusal: show it.
    Fail,
    /// Twitch was busy with this user, or the old one just ran out: send it again.
    Retry,
    /// One is in place, but which isn't known yet: look it up, then ask again.
    LookUp,
    /// Lift the current timeout, then apply ours. Only the last one counts.
    Replace,
    /// A timeout over a permanent ban: the ban stays, say so.
    KeepBan,
}

/// Whether Twitch refused because the user is already banned or timed out.
fn already(r: Refusal) -> bool {
    match r {
        Refusal::Helix { status, message } => status == 400 && message.to_ascii_lowercase().contains("already banned"),
        Refusal::Gql(code) => code == "TARGET_ALREADY_BANNED" || code == "ALREADY_BANNED",
    }
}

/// The plan for a refused ban (`timeout: false`) or timeout. `held` is the looked-up
/// state, `None` before the lookup.
pub fn plan(timeout: bool, r: Refusal, held: Option<Held>) -> Plan {
    if matches!(r, Refusal::Helix { status: 409, .. }) {
        return Plan::Retry;
    }
    if !already(r) {
        return Plan::Fail;
    }
    // Helix replaces a timeout itself, so "already banned" there means a permanent ban.
    let held = match (r, held) {
        (_, Some(h)) => h,
        (Refusal::Helix { .. }, None) => Held::Ban,
        (Refusal::Gql(_), None) => return Plan::LookUp,
    };
    match (held, timeout) {
        (Held::Nothing, _) => Plan::Retry,
        (Held::Timeout, _) => Plan::Replace,
        (Held::Ban, true) => Plan::KeepBan,
        (Held::Ban, false) => Plan::Fail,
    }
}

/// The line for [`Plan::KeepBan`].
pub fn keep_ban_line(who: &str) -> String {
    format!("{who} is banned. Unban them first to time them out.")
}

/// The line when our lift went through but the new one didn't: they are free now.
pub fn lifted_line(who: &str, why: &str) -> String {
    format!("Lifted {who}’s timeout, but couldn’t apply the new one: {}.", why.trim_end_matches('.'))
}

/// Most sends of one ban: the first, a retry after a busy answer, a lift and the replacement.
/// One more send always follows a lift.
pub const TRIES: usize = 4;

/// A refused send, owned: how Twitch said no (`None`: not a Twitch answer) and the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub how: Option<How>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum How {
    Helix { status: u16, message: String },
    Gql(String),
}

impl Refused {
    fn refusal(&self) -> Option<Refusal<'_>> {
        self.how.as_ref().map(|h| match h {
            How::Helix { status, message } => Refusal::Helix { status: *status, message },
            How::Gql(code) => Refusal::Gql(code),
        })
    }
}

/// What one ban needs from Twitch. The app talks to it; tests script it.
/// Its futures are `Send`, so a Tauri command can await them.
pub trait Twitch: Send {
    /// One send of the ban or timeout.
    fn send(&mut self) -> impl Future<Output = Result<(), Refused>> + Send;
    /// What they have now; `None` when it can't be read (or isn't needed: Helix).
    fn held(&mut self) -> impl Future<Output = Option<Held>> + Send;
    /// Lifts the current timeout, marking its echo as ours. `Err` is the line.
    fn lift(&mut self) -> impl Future<Output = Result<(), String>> + Send;
    /// The lift went through, the new one didn't: its echo gets a line after all.
    fn unmark_lift(&mut self);
    /// A short wait before sending again.
    fn pause(&mut self) -> impl Future<Output = ()> + Send;
}

/// V107-M7 Bans (`timeout: false`) or times out so the last one counts: retries while
/// Twitch is busy, lifts a timeout in the way once, and always sends again after that.
/// A stale "already banned" after our lift waits instead of lifting twice.
pub async fn run(tw: &mut impl Twitch, who: &str, timeout: bool) -> Result<(), String> {
    run_full(tw, who, timeout).await.map_err(|f| f.text)
}

/// A ban that didn’t go through: the line, and whether one is in place and stays (a ban
/// over a ban, a timeout over a ban). Mass actions count that as "already" by this, not by
/// the line’s words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub text: String,
    pub in_place: bool,
}

/// [`run`], telling an in-place ban apart from a refusal.
pub async fn run_full(tw: &mut impl Twitch, who: &str, timeout: bool) -> Result<(), Failure> {
    let mut lifted = false;
    let mut sent_after_lift = false;
    let mut sends = 0;
    let mut last = None;
    while sends < TRIES || (lifted && !sent_after_lift) {
        sends += 1;
        sent_after_lift = lifted;
        let Err(no) = tw.send().await else { return Ok(()) };
        let Some(r) = no.refusal() else { return fail(tw, who, lifted, no.text, false) };
        let mut plan = plan(timeout, r, None);
        // A Fail on an "already" answer means a ban stays; unknown when the lookup failed.
        let mut known = true;
        if plan == Plan::LookUp {
            plan = match tw.held().await {
                Some(h) => self::plan(timeout, r, Some(h)),
                None => {
                    known = false;
                    Plan::Fail
                }
            };
        }
        match plan {
            Plan::Retry => tw.pause().await,
            // Twitch hasn't caught up with our lift yet.
            Plan::Replace if lifted => tw.pause().await,
            Plan::Replace => {
                tw.lift().await.map_err(|text| Failure { text, in_place: false })?;
                lifted = true;
                sent_after_lift = false;
            }
            Plan::KeepBan => return fail(tw, who, lifted, keep_ban_line(who), true),
            Plan::Fail | Plan::LookUp => {
                let in_place = known && already(r) && plan == Plan::Fail;
                return fail(tw, who, lifted, no.text, in_place);
            }
        }
        last = Some(no.text);
    }
    fail(tw, who, lifted, last.unwrap_or_else(|| "Twitch refused it.".into()), false)
}

/// After our lift they are free: never in place then.
fn fail(tw: &mut impl Twitch, who: &str, lifted: bool, why: String, in_place: bool) -> Result<(), Failure> {
    if !lifted {
        return Err(Failure { text: why, in_place });
    }
    tw.unmark_lift();
    Err(Failure { text: lifted_line(who, &why), in_place: false })
}

/// V107-M7 A mass ban's refusal worth one ban on its own: one is in place (the last one
/// counts, `run` decides) or Twitch was busy. GQL's batch answers by code.
pub fn redo(timeout: bool, code: &str) -> bool {
    plan(timeout, Refusal::Gql(code), None) != Plan::Fail
}

/// The items in order until `stop` says so (asked before each), for a mass action's Stop.
pub fn until<'a, T>(items: &'a [T], stop: impl Fn() -> bool + 'a) -> impl Iterator<Item = &'a T> {
    items.iter().take_while(move |_| !stop())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GQL: Refusal = Refusal::Gql("TARGET_ALREADY_BANNED");
    const HELIX: Refusal = Refusal::Helix { status: 400, message: "The user specified in the user_id field is already banned." };

    #[test]
    fn v107_m7_gql_looks_up_then_replaces_a_timeout() {
        assert_eq!(plan(true, GQL, None), Plan::LookUp);
        assert_eq!(plan(true, GQL, Some(Held::Timeout)), Plan::Replace);
        // A ban over a timeout also wins.
        assert_eq!(plan(false, GQL, None), Plan::LookUp);
        assert_eq!(plan(false, GQL, Some(Held::Timeout)), Plan::Replace);
    }

    #[test]
    fn v107_m7_a_permanent_ban_stays() {
        assert_eq!(plan(true, GQL, Some(Held::Ban)), Plan::KeepBan);
        assert_eq!(plan(true, HELIX, None), Plan::KeepBan);
        assert_eq!(plan(false, GQL, Some(Held::Ban)), Plan::Fail, "already banned");
        assert_eq!(plan(false, HELIX, None), Plan::Fail);
        assert_eq!(keep_ban_line("caressz"), "caressz is banned. Unban them first to time them out.");
    }

    #[test]
    fn v107_m7_busy_or_expired_retries() {
        assert_eq!(plan(true, Refusal::Helix { status: 409, message: "" }, None), Plan::Retry);
        assert_eq!(plan(true, GQL, Some(Held::Nothing)), Plan::Retry);
    }

    #[test]
    fn v107_m7_other_refusals_fail() {
        for r in [
            Refusal::Gql("TARGET_IS_MOD"),
            Refusal::Gql("FORBIDDEN"),
            Refusal::Helix { status: 400, message: "The user specified in the user_id field may not be banned." },
            Refusal::Helix { status: 403, message: "" },
            Refusal::Helix { status: 429, message: "" },
        ] {
            assert_eq!(plan(true, r, None), Plan::Fail, "{r:?}");
            assert_eq!(plan(true, r, Some(Held::Timeout)), Plan::Fail, "{r:?}");
        }
    }

    /// Polls a future that never waits for real (the scripted Twitch answers at once).
    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        let mut f = std::pin::pin!(f);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    /// Scripted Twitch: one answer per send, then what the lookup says.
    #[derive(Default)]
    struct Script {
        sends: Vec<Result<(), Refused>>,
        held: Vec<Held>,
        lift: Vec<Result<(), String>>,
        log: Vec<&'static str>,
    }

    fn gql(code: &str) -> Result<(), Refused> {
        Err(Refused { how: Some(How::Gql(code.into())), text: format!("{code}.") })
    }

    fn busy() -> Result<(), Refused> {
        Err(Refused { how: Some(How::Helix { status: 409, message: String::new() }), text: "Busy.".into() })
    }

    impl Twitch for Script {
        async fn send(&mut self) -> Result<(), Refused> {
            self.log.push("send");
            if self.sends.is_empty() { Ok(()) } else { self.sends.remove(0) }
        }
        async fn held(&mut self) -> Option<Held> {
            self.log.push("held");
            (!self.held.is_empty()).then(|| self.held.remove(0))
        }
        async fn lift(&mut self) -> Result<(), String> {
            self.log.push("lift");
            if self.lift.is_empty() { Ok(()) } else { self.lift.remove(0) }
        }
        fn unmark_lift(&mut self) {
            self.log.push("unmark");
        }
        async fn pause(&mut self) {
            self.log.push("pause");
        }
    }

    const ALREADY: &str = "TARGET_ALREADY_BANNED";

    #[test]
    fn v107_m7_replaces_a_timeout_once() {
        let mut s = Script { sends: vec![gql(ALREADY)], held: vec![Held::Timeout], ..Default::default() };
        assert_eq!(block_on(run(&mut s, "caressz", true)), Ok(()));
        assert_eq!(s.log, ["send", "held", "lift", "send"]);
    }

    #[test]
    fn v107_m7_a_lift_on_the_last_pass_still_sends() {
        // Busy three times, then the lift on pass 4: a fifth send puts the new one on.
        let mut s = Script { sends: vec![busy(), busy(), busy(), gql(ALREADY)], held: vec![Held::Timeout], ..Default::default() };
        assert_eq!(block_on(run(&mut s, "caressz", true)), Ok(()));
        assert_eq!(s.log.iter().filter(|l| **l == "send").count(), TRIES + 1);
        assert_eq!(s.log.last(), Some(&"send"));
        // And when that one fails, they're free: say so, and the lift's echo gets its line.
        let mut s = Script {
            sends: vec![busy(), busy(), busy(), gql(ALREADY), busy()],
            held: vec![Held::Timeout],
            ..Default::default()
        };
        assert_eq!(
            block_on(run(&mut s, "caressz", true)),
            Err("Lifted caressz’s timeout, but couldn’t apply the new one: Busy.".into())
        );
        assert_eq!(s.log.last(), Some(&"unmark"));
    }

    #[test]
    fn v107_m7_a_stale_already_after_the_lift_waits() {
        // Twitch still reports the old timeout after our lift: wait and send, never unban twice.
        let mut s = Script {
            sends: vec![gql(ALREADY), gql(ALREADY)],
            held: vec![Held::Timeout, Held::Timeout],
            ..Default::default()
        };
        assert_eq!(block_on(run(&mut s, "caressz", false)), Ok(()));
        assert_eq!(s.log, ["send", "held", "lift", "send", "held", "pause", "send"]);
        // Stale every time: out of tries, they're free, the line says it.
        let mut s = Script { sends: vec![gql(ALREADY); 6], held: vec![Held::Timeout; 6], ..Default::default() };
        let err = block_on(run(&mut s, "caressz", true)).unwrap_err();
        assert_eq!(err, lifted_line("caressz", "TARGET_ALREADY_BANNED."));
        assert_eq!(s.log.iter().filter(|l| **l == "lift").count(), 1);
        assert_eq!(s.log.iter().filter(|l| **l == "send").count(), TRIES);
        assert_eq!(s.log.last(), Some(&"unmark"));
    }

    #[test]
    fn v107_m7_failures_without_a_lift_say_twitchs_line() {
        let mut s = Script { sends: vec![gql("TARGET_IS_MOD")], ..Default::default() };
        assert_eq!(block_on(run(&mut s, "x", true)), Err("TARGET_IS_MOD.".into()));
        let mut s = Script { sends: vec![gql(ALREADY)], held: vec![Held::Ban], ..Default::default() };
        assert_eq!(block_on(run(&mut s, "x", true)), Err(keep_ban_line("x")));
        // Lookup failed: Twitch's line, nothing lifted.
        let mut s = Script { sends: vec![gql(ALREADY)], ..Default::default() };
        assert_eq!(block_on(run(&mut s, "x", true)), Err("TARGET_ALREADY_BANNED.".into()));
        // The lift itself failed: its line, and `lift` undid its own mark.
        let mut s = Script { sends: vec![gql(ALREADY)], held: vec![Held::Timeout], lift: vec![Err("No.".into())], ..Default::default() };
        assert_eq!(block_on(run(&mut s, "x", true)), Err("No.".into()));
        assert!(!s.log.contains(&"unmark"));
        // Busy every time.
        let mut s = Script { sends: vec![busy(); TRIES], ..Default::default() };
        assert_eq!(block_on(run(&mut s, "x", true)), Err("Busy.".into()));
    }

    #[test]
    fn v107_m7_mass_bans_redo_what_is_in_place() {
        assert!(redo(true, ALREADY), "timeout over a timeout");
        assert!(redo(false, ALREADY), "a ban replaces a timeout too");
        assert!(redo(false, "ALREADY_BANNED"));
        assert!(!redo(true, "TARGET_IS_MOD"));
        assert!(!redo(false, "USER_NOT_FOUND"));
    }

    #[test]
    fn v107_m7_until_stops_when_told() {
        let stop = std::cell::Cell::new(false);
        let mut seen = Vec::new();
        for n in until(&[1, 2, 3, 4], || stop.get()) {
            seen.push(*n);
            if *n == 2 {
                stop.set(true);
            }
        }
        assert_eq!(seen, [1, 2]);
        assert_eq!(until(&[1], || true).count(), 0);
    }

    #[test]
    fn v107_r21_in_place_is_told_by_the_code_not_the_words() {
        let full = |sends: Vec<Result<(), Refused>>, held: Vec<Held>, timeout: bool| {
            let mut s = Script { sends, held, ..Default::default() };
            block_on(run_full(&mut s, "x", timeout)).unwrap_err()
        };
        // A ban over a ban, a timeout over a ban: in place.
        assert!(full(vec![gql(ALREADY)], vec![Held::Ban], false).in_place);
        assert!(full(vec![gql(ALREADY)], vec![Held::Ban], true).in_place);
        let helix = Refused {
            how: Some(How::Helix { status: 400, message: "The user specified in the user_id field is already banned.".into() }),
            text: "x".into(),
        };
        assert!(full(vec![Err(helix)], vec![], false).in_place);
        // "isn't banned" and other refusals are not, whatever the line says.
        let not = Refused { how: Some(How::Gql("TARGET_NOT_BANNED".into())), text: "x isn’t banned.".into() };
        assert!(!full(vec![Err(not)], vec![], false).in_place);
        assert!(!full(vec![gql("TARGET_IS_MOD")], vec![], false).in_place);
        // Lookup failed: unknown, not counted as in place.
        assert!(!full(vec![gql(ALREADY)], vec![], false).in_place);
    }
}
