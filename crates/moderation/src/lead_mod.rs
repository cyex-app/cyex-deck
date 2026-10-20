//! V107-R28 `/leadmod` and `/unleadmod`: the broadcaster gives or takes the lead moderator
//! role. GQL only (`assignChannelRole`, `roleID: "lead_mod"`), like twitch.tv's web chat;
//! Helix has no endpoint. The sequence runs here against a small port ([`LeadOps`]).

/// No GQL sign-in: Helix can't do it.
pub const NEEDS_GQL: &str = "Lead mods are set with a GQL sign-in. Add one in Settings → Accounts.";

/// The line after it went through.
pub fn done(who: &str, on: bool) -> String {
    if on {
        format!("You made {who} lead moderator.")
    } else {
        format!("{who} is no longer lead moderator.")
    }
}

/// What a refusal from Twitch reads as; unknown codes show the code.
pub fn refused(code: &str, who: &str, on: bool) -> String {
    match code.to_ascii_uppercase().as_str() {
        "ALREADY_ASSIGNED" => format!("{who} is already lead moderator."),
        "NOT_ASSIGNED" => format!("{who} isn’t lead moderator."),
        "PERMISSION_DENIED" => "Only the broadcaster can do that.".into(),
        "ROLE_COUNT_LIMIT_REACHED" => "No lead mod slots left.".into(),
        "USER_IS_BANNED" => format!("{who} is banned here."),
        c if c == "CANNOT_BE_ASSIGNED" || c.starts_with("CANNOT_ASSIGN") => format!("{who} can’t be lead moderator."),
        "NOT_DONE" => format!("Twitch didn’t {} it. Try again.", if on { "do" } else { "undo" }),
        other => format!("Twitch refused it ({other})."),
    }
}

/// What the sequence needs from the app: Twitch, for one broadcaster in their channel.
pub trait LeadOps: Sync {
    /// Whether there is a GQL sign-in.
    fn can(&self) -> bool;
    /// A login's user id; `Ok(None)` when nobody has it.
    fn user_id(&self, login: &str) -> impl Future<Output = Result<Option<String>, String>> + Send;
    /// Gives (`on`) or takes the role. `Ok(Some(code))`: Twitch refused.
    fn assign(&self, target_id: &str, on: bool) -> impl Future<Output = Result<Option<String>, String>> + Send;
}

/// Gives or takes lead mod. `Ok` is the chat line; `Err` the line to show as an error.
pub async fn set_lead(ops: &impl LeadOps, who: &str, on: bool) -> Result<String, String> {
    if !ops.can() {
        return Err(NEEDS_GQL.into());
    }
    let id = ops.user_id(who).await?.ok_or_else(|| format!("No user called {who}."))?;
    match ops.assign(&id, on).await? {
        None => Ok(done(who, on)),
        Some(code) => Err(refused(&code, who, on)),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Scripted Twitch: a user id and one answer to the mutation.
    struct Fake {
        can: bool,
        id: Result<Option<String>, String>,
        answer: Result<Option<String>, String>,
        sent: Mutex<Vec<(String, bool)>>,
    }

    fn fake(answer: Result<Option<&str>, &str>) -> Fake {
        Fake {
            can: true,
            id: Ok(Some("42".into())),
            answer: answer.map(|c| c.map(Into::into)).map_err(Into::into),
            sent: Mutex::default(),
        }
    }

    impl LeadOps for Fake {
        fn can(&self) -> bool {
            self.can
        }

        fn user_id(&self, _login: &str) -> impl Future<Output = Result<Option<String>, String>> + Send {
            std::future::ready(self.id.clone())
        }

        fn assign(&self, target_id: &str, on: bool) -> impl Future<Output = Result<Option<String>, String>> + Send {
            self.sent.lock().unwrap().push((target_id.into(), on));
            std::future::ready(self.answer.clone())
        }
    }

    /// Every future here is ready at once, so one poll runs it to the end.
    fn run(f: &Fake, on: bool) -> Result<String, String> {
        let mut fut = std::pin::pin!(set_lead(f, "caressz", on));
        match fut.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
            std::task::Poll::Ready(r) => r,
            std::task::Poll::Pending => panic!("the fake never waits"),
        }
    }

    #[test]
    fn v107_r28_success_lines() {
        let f = fake(Ok(None));
        assert_eq!(run(&f, true), Ok("You made caressz lead moderator.".into()));
        assert_eq!(run(&f, false), Ok("caressz is no longer lead moderator.".into()));
        assert_eq!(*f.sent.lock().unwrap(), [("42".to_owned(), true), ("42".to_owned(), false)]);
    }

    #[test]
    fn v107_r28_each_error_code() {
        let cases = [
            ("ALREADY_ASSIGNED", true, "caressz is already lead moderator."),
            ("PERMISSION_DENIED", true, "Only the broadcaster can do that."),
            ("ROLE_COUNT_LIMIT_REACHED", true, "No lead mod slots left."),
            ("USER_IS_BANNED", true, "caressz is banned here."),
            ("CANNOT_BE_ASSIGNED", true, "caressz can’t be lead moderator."),
            ("CANNOT_ASSIGN_TO_SELF", true, "caressz can’t be lead moderator."),
            ("NOT_ASSIGNED", false, "caressz isn’t lead moderator."),
            ("NOT_DONE", true, "Twitch didn’t do it. Try again."),
            ("SOMETHING_NEW", false, "Twitch refused it (SOMETHING_NEW)."),
        ];
        for (code, on, line) in cases {
            assert_eq!(run(&fake(Ok(Some(code))), on), Err(line.into()), "{code}");
        }
    }

    #[test]
    fn v107_r28_no_sign_in_no_user_or_network_sends_nothing_more() {
        let mut f = fake(Ok(None));
        f.can = false;
        assert_eq!(run(&f, true), Err(NEEDS_GQL.into()));
        assert!(f.sent.lock().unwrap().is_empty());
        let mut f = fake(Ok(None));
        f.id = Ok(None);
        assert_eq!(run(&f, true), Err("No user called caressz.".into()));
        assert!(f.sent.lock().unwrap().is_empty());
        let f = fake(Err("Twitch is down."));
        assert_eq!(run(&f, false), Err("Twitch is down.".into()));
    }
}
