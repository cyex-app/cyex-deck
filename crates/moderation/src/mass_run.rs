//! V107-R21 The mass ban loops, with Twitch behind a trait so tests script it: GQL in
//! batches, a gap between requests, throttled logins sent again with backoff; Helix one
//! login per request, waiting for Twitch's reset time after a 429.

use crate::mass::{self, Outcome, Step, Tally};

/// A whole request that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// 429.
    Throttled,
    /// 5xx, offline, a timeout: worth another go.
    Busy,
    /// The token stopped working: stop the run.
    Unauthorized,
    /// Anything else: stop with this line.
    Fatal(String),
}

/// What the loops need from the app.
pub trait Io: Send {
    fn wait(&mut self, ms: i64) -> impl Future<Output = ()> + Send;
    fn stopped(&self) -> bool;
    fn progress(&mut self, tally: &Tally);
}

/// One GQL batch: each login with its code or entry error (`None` went through).
pub trait Batches: Io {
    fn send(&mut self, logins: &[String]) -> impl Future<Output = Result<Vec<(String, Option<String>)>, Fault>> + Send;
}

/// A Helix answer: `Err((status, message))` (status 0 offline), and the wait Twitch's
/// rate-limit headers ask for next.
pub type Answer = (Result<(), (u16, String)>, i64);

/// One Helix request for one user id.
pub trait Singles: Io {
    fn send(&mut self, id: &str) -> impl Future<Output = Answer> + Send;
}

/// One login lookup: `(login, id)` for each login Twitch knows.
pub trait Lookups: Io {
    fn lookup(&mut self, logins: &[String]) -> impl Future<Output = Result<Vec<(String, String)>, Fault>> + Send;
}

const LOST: &str = "Lost permission mid-ban.";
const LOOKUP_BUSY: &str = "Twitch is busy. Try again in a moment.";
/// Logins per lookup (Twitch's cap for both Helix `/users` and GQL `users`).
pub const LOOKUP_CHUNK: usize = 100;

/// V107-R21 The logins Twitch knows, with their ids: [`LOOKUP_CHUNK`] a call,
/// [`mass::GQL_GAP_MS`] between calls, busy answers sent again with backoff. Gone ones go
/// into the tally. A lookup that keeps failing stops the run when `strict` (Helix needs
/// ids), else that chunk goes on unchecked with empty ids (GQL bans by login).
pub async fn resolve(io: &mut impl Lookups, logins: &[String], strict: bool, tally: &mut Tally) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::with_capacity(logins.len());
    for (i, chunk) in logins.chunks(LOOKUP_CHUNK).enumerate() {
        if io.stopped() {
            break;
        }
        if i > 0 {
            io.wait(mass::GQL_GAP_MS).await;
        }
        let mut n = 0;
        let found = loop {
            match io.lookup(chunk).await {
                Ok(f) => break Some(f),
                Err(Fault::Unauthorized) => return Err(LOST.into()),
                Err(Fault::Throttled | Fault::Busy) if n + 1 < mass::TRIES => {
                    n += 1;
                    io.wait(mass::backoff_ms(n, None)).await;
                }
                Err(Fault::Fatal(t)) if strict => return Err(t),
                Err(_) if strict => return Err(LOOKUP_BUSY.into()),
                Err(_) => break None,
            }
        };
        let Some(found) = found else {
            out.extend(chunk.iter().map(|l| (l.clone(), String::new())));
            continue;
        };
        let ids: std::collections::HashMap<String, String> =
            found.into_iter().map(|(l, id)| (l.to_ascii_lowercase(), id)).collect();
        for login in chunk {
            match ids.get(&login.to_ascii_lowercase()) {
                Some(id) => out.push((login.clone(), id.clone())),
                None => tally.add(Outcome::Gone, None),
            }
        }
    }
    io.progress(tally);
    Ok(out)
}

/// Bans the logins `batch` at a time, [`mass::GQL_GAP_MS`] between requests. What came
/// back "already banned" and needs the single-ban path (V107-M7) is returned, in order.
pub async fn batches(
    io: &mut impl Batches,
    logins: &[String],
    batch: usize,
    timeout: bool,
    unban: bool,
    tally: &mut Tally,
) -> Result<Vec<String>, String> {
    let mut again = Vec::new();
    let mut first = true;
    for chunk in logins.chunks(batch.max(1)) {
        let mut pending: Vec<String> = chunk.to_vec();
        let mut n = 0;
        while !pending.is_empty() {
            if !first {
                io.wait(mass::GQL_GAP_MS).await;
            }
            // After the gap too: a Stop pressed while waiting sends nothing more.
            if io.stopped() {
                return Ok(again);
            }
            first = false;
            let mut retry = Vec::new();
            let last_try = n + 1 >= mass::TRIES;
            match io.send(&pending).await {
                Ok(results) => {
                    for (login, code) in results {
                        match mass::gql_step(code.as_deref(), timeout, unban) {
                            Step::End(o, c) => tally.add(o, c.as_deref()),
                            Step::Retry if !last_try => {
                                retry.push(login);
                            }
                            Step::Retry => tally.add(Outcome::Throttled, None),
                            Step::Restack => again.push(login),
                        }
                    }
                }
                Err(Fault::Unauthorized) => return Err(LOST.into()),
                Err(Fault::Fatal(t)) => return Err(t),
                Err(_) if !last_try => retry = pending.clone(),
                Err(f) => {
                    let o = if f == Fault::Throttled { Outcome::Throttled } else { Outcome::Other };
                    for _ in &pending {
                        tally.add(o, Some("network"));
                    }
                }
            }
            io.progress(tally);
            if !retry.is_empty() {
                n += 1;
                tally.retries += retry.len() as u32;
                io.wait(mass::backoff_ms(n, None)).await;
            }
            pending = retry;
        }
    }
    Ok(again)
}

/// Bans the ids one by one, paced by Twitch's headers, a 429 waits for the reset.
pub async fn singles(io: &mut impl Singles, ids: &[String], unban: bool, tally: &mut Tally) -> Result<(), String> {
    for id in ids {
        let mut n = 0;
        loop {
            if io.stopped() {
                return Ok(());
            }
            let (res, pace) = io.send(id).await;
            let step = match &res {
                Ok(()) => Step::End(Outcome::Done, None),
                Err((401, _)) => return Err(LOST.into()),
                Err((0, _)) => Step::Retry,
                Err((s, m)) => mass::helix_step(*s, m, unban),
            };
            let limited = matches!(res, Err((429, _)));
            match step {
                Step::Retry if n + 1 < mass::TRIES => {
                    n += 1;
                    tally.retries += 1;
                    io.wait(mass::backoff_ms(n, limited.then_some(pace))).await;
                    continue;
                }
                Step::Retry => tally.add(if limited { Outcome::Throttled } else { Outcome::Other }, Some("busy")),
                Step::End(o, c) => tally.add(o, c.as_deref()),
                Step::Restack => tally.add(Outcome::Already, None),
            }
            io.progress(tally);
            io.wait(pace).await;
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    /// Scripted Twitch: answers in order, records the sends and waits.
    #[derive(Default)]
    struct Fake {
        batches: VecDeque<Result<Vec<Option<&'static str>>, Fault>>,
        singles: VecDeque<Answer>,
        lookups: VecDeque<Result<Vec<&'static str>, Fault>>,
        stop_after: Option<usize>,
        /// Stop is pressed during the next wait.
        stop_in_wait: bool,
        stopped_now: bool,
        sent: Vec<Vec<String>>,
        waits: Vec<i64>,
    }

    impl Io for Fake {
        async fn wait(&mut self, ms: i64) {
            self.waits.push(ms);
            self.stopped_now |= self.stop_in_wait;
        }
        fn stopped(&self) -> bool {
            self.stopped_now || self.stop_after.is_some_and(|n| self.sent.len() >= n)
        }
        fn progress(&mut self, _: &Tally) {}
    }

    impl Batches for Fake {
        async fn send(&mut self, logins: &[String]) -> Result<Vec<(String, Option<String>)>, Fault> {
            self.sent.push(logins.to_vec());
            let codes = self.batches.pop_front().unwrap_or_else(|| Ok(vec![None; logins.len()]))?;
            Ok(logins.iter().cloned().zip(codes.into_iter().map(|c| c.map(str::to_owned))).collect())
        }
    }

    impl Lookups for Fake {
        async fn lookup(&mut self, logins: &[String]) -> Result<Vec<(String, String)>, Fault> {
            self.sent.push(logins.to_vec());
            match self.lookups.pop_front() {
                // Twitch knows everyone but these, ids as `id-<login>`, case as Twitch has it.
                Some(Ok(gone)) => Ok(logins.iter().filter(|l| !gone.contains(&l.as_str())).map(|l| (l.to_uppercase(), format!("id-{l}"))).collect()),
                Some(Err(f)) => Err(f),
                None => Ok(logins.iter().map(|l| (l.clone(), format!("id-{l}"))).collect()),
            }
        }
    }

    impl Singles for Fake {
        async fn send(&mut self, id: &str) -> Answer {
            self.sent.push(vec![id.to_owned()]);
            self.singles.pop_front().unwrap_or((Ok(()), 100))
        }
    }

    /// Polls a future that never waits for real (the fake answers at once).
    fn block_on<F: Future>(f: F) -> F::Output {
        let mut f = std::pin::pin!(f);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    fn names(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("u{i}")).collect()
    }

    #[test]
    fn gql_throttled_logins_go_again_after_a_backoff() {
        let mut f = Fake::default();
        f.batches.push_back(Ok(vec![None, Some("RATE_LIMITED"), Some("TARGET_NOT_FOUND")]));
        let mut t = Tally::default();
        let again = block_on(batches(&mut f, &names(3), 30, false, false, &mut t)).unwrap();
        assert!(again.is_empty());
        assert_eq!(f.sent, vec![names(3), vec!["u1".to_owned()]]);
        assert_eq!((t.done, t.gone, t.retries), (2, 1, 1));
        // The backoff, then the usual gap before the next request.
        assert_eq!(f.waits, vec![2_000, mass::GQL_GAP_MS]);
    }

    #[test]
    fn gql_whole_batch_429_retries_then_gives_up_as_rate_limited() {
        let mut f = Fake::default();
        for _ in 0..mass::TRIES {
            f.batches.push_back(Err(Fault::Throttled));
        }
        let mut t = Tally::default();
        block_on(batches(&mut f, &names(2), 30, false, false, &mut t)).unwrap();
        assert_eq!(f.sent.len(), mass::TRIES as usize);
        assert_eq!((t.throttled, t.retries), (2, 2 * (mass::TRIES - 1)));
    }

    #[test]
    fn gql_paces_batches_and_hands_back_already_banned() {
        let mut f = Fake::default();
        f.batches.push_back(Ok(vec![Some("TARGET_ALREADY_BANNED"), Some("TARGET_IS_MOD")]));
        let mut t = Tally::default();
        let again = block_on(batches(&mut f, &names(5), 2, false, false, &mut t)).unwrap();
        assert_eq!(again, vec!["u0"]);
        assert_eq!(f.sent.iter().map(Vec::len).collect::<Vec<_>>(), vec![2, 2, 1]);
        assert_eq!(f.waits, vec![mass::GQL_GAP_MS; 2]);
        assert_eq!((t.done, t.protected), (3, 1));
    }

    #[test]
    fn gql_keeps_one_request_a_second_clean_or_not() {
        let mut f = Fake::default();
        for _ in 0..4 {
            f.batches.push_back(Ok(vec![None]));
        }
        f.batches.push_back(Ok(vec![Some("RATE_LIMITED")]));
        let mut t = Tally::default();
        block_on(batches(&mut f, &names(8), 1, false, false, &mut t)).unwrap();
        let g = mass::GQL_GAP_MS;
        // V107-R21 No speed-up after clean answers: 1 s always, the throttle backs off.
        assert_eq!(f.waits, vec![g, g, g, g, 2_000, g, g, g, g]);
        assert_eq!((t.done, t.retries), (8, 1));
    }

    #[test]
    fn gql_lost_token_stops() {
        let mut f = Fake::default();
        f.batches.push_back(Err(Fault::Unauthorized));
        let mut t = Tally::default();
        assert_eq!(block_on(batches(&mut f, &names(2), 30, false, false, &mut t)).unwrap_err(), LOST);
    }

    #[test]
    fn helix_429_waits_for_the_reset_and_sends_again() {
        let mut f = Fake::default();
        f.singles.push_back((Err((429, "Too Many Requests".into())), 12_000));
        f.singles.push_back((Ok(()), 100));
        f.singles.push_back((Err((400, "The user specified in the user_id field is already banned.".into())), 100));
        let mut t = Tally::default();
        block_on(singles(&mut f, &["1".to_owned(), "2".to_owned()], false, &mut t)).unwrap();
        assert_eq!(f.sent, vec![vec!["1".to_owned()], vec!["1".to_owned()], vec!["2".to_owned()]]);
        assert_eq!(f.waits, vec![12_000, 100, 100]);
        assert_eq!((t.done, t.already, t.retries), (1, 1, 1));
    }

    #[test]
    fn helix_still_throttled_counts_as_rate_limited() {
        let mut f = Fake::default();
        for _ in 0..mass::TRIES {
            f.singles.push_back((Err((429, String::new())), 0));
        }
        let mut t = Tally::default();
        block_on(singles(&mut f, &["1".to_owned()], false, &mut t)).unwrap();
        assert_eq!(t.throttled, 1);
        // Doubling waits when the reset is already past.
        assert_eq!(&f.waits[..4], &[2_000, 4_000, 8_000, 16_000]);
    }

    #[test]
    fn v107_r21_lookups_are_paced_and_count_the_gone() {
        let mut f = Fake::default();
        f.lookups.push_back(Ok(vec!["u1"]));
        let mut t = Tally::default();
        let out = block_on(resolve(&mut f, &names(250), true, &mut t)).unwrap();
        assert_eq!(f.sent.iter().map(Vec::len).collect::<Vec<_>>(), [100, 100, 50]);
        // One lookup a second, none before the first.
        assert_eq!(f.waits, [mass::GQL_GAP_MS; 2]);
        assert_eq!((out.len(), t.gone), (249, 1));
        assert_eq!(out[0], ("u0".to_owned(), "id-u0".to_owned()), "matched case-blind");
    }

    #[test]
    fn v107_r21_a_failing_lookup_retries_then_stops_or_goes_unchecked() {
        let busy = || {
            let mut f = Fake::default();
            for _ in 0..mass::TRIES {
                f.lookups.push_back(Err(Fault::Busy));
            }
            f
        };
        let mut f = busy();
        let mut t = Tally::default();
        assert_eq!(block_on(resolve(&mut f, &names(2), true, &mut t)).unwrap_err(), LOOKUP_BUSY);
        assert_eq!(f.waits, [2_000, 4_000, 8_000, 16_000]);
        // GQL bans by login: sent unchecked.
        let mut f = busy();
        let out = block_on(resolve(&mut f, &names(2), false, &mut t)).unwrap();
        assert_eq!(out, [("u0".to_owned(), String::new()), ("u1".to_owned(), String::new())]);
        let mut f = Fake::default();
        f.lookups.push_back(Err(Fault::Unauthorized));
        assert_eq!(block_on(resolve(&mut f, &names(2), false, &mut t)).unwrap_err(), LOST);
        let mut f = Fake::default();
        f.lookups.push_back(Err(Fault::Fatal("Bad.".into())));
        assert_eq!(block_on(resolve(&mut f, &names(2), true, &mut t)).unwrap_err(), "Bad.");
    }

    #[test]
    fn v107_r21_stop_ends_the_lookups() {
        let mut f = Fake { stop_after: Some(1), ..Default::default() };
        let mut t = Tally::default();
        let out = block_on(resolve(&mut f, &names(300), true, &mut t)).unwrap();
        assert_eq!((f.sent.len(), out.len()), (1, 100));
    }

    /// A Stop pressed during the gap between batches sends nothing more (live test 2026/10/21).
    #[test]
    fn stop_during_the_gap_sends_no_more_batches() {
        let mut io = Fake { stop_in_wait: true, ..Fake::default() };
        let mut tally = Tally::default();
        block_on(batches(&mut io, &names(60), 30, false, false, &mut tally)).unwrap();
        assert_eq!(io.sent.len(), 1, "only the batch before the gap went out");
    }
}
