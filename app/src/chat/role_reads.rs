//! V111-12 Global mentions without Twitch's chat delay: channels where the default account
//! moderates (lead mods too) or broadcasts, and has no chat open, are read in the background
//! on a connection of their own, signed in as that account. Twitch doesn't delay chat for mods
//! and the broadcaster, and a mod or broadcaster room doesn't count toward the 100-chat limit.
//! Only for matching: lines go to `global::irc_line` and nowhere else (no view, no history,
//! no join or leave lines). The firehose's later copy of a message is dropped by id.
//!
//! Roles come from Twitch's `moderatedChannels` (GQL, newest role first), else Helix's
//! moderated channels (`user:read:moderated_channels`), every 30 min. At most
//! [`MAX_CHANNELS`]; joins go at [`JOIN_LIMIT`] per 10 s so the account's own chats keep their
//! 18 of Twitch's 20. Runs while global mentions are on and have something to find; stops when
//! they turn off, the default account changes or signs out.

use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cyex_chat::irc::{self, Client, Event, LinkStatus};
use cyex_twitch::gql::dashboard::community;
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, watch};

use super::creds::{AccountCreds, WriterCreds};

/// Most role channels read at once.
pub const MAX_CHANNELS: usize = 100;
/// JOINs per 10 s on this connection. With the account's chats at `Window::JOIN_LIMIT`, the
/// account stays at Twitch's 20.
pub const JOIN_LIMIT: usize = 2;
/// Roles are read again this often.
const REFRESH: Duration = Duration::from_secs(30 * 60);
/// Chats opened or closed are picked up this often (an open chat reads its own lines).
const REPLAN: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct RoleReads {
    running: Mutex<Option<Running>>,
}

struct Running {
    account: String,
    stop: watch::Sender<bool>,
}

pub fn init(app: &AppHandle) {
    app.manage(RoleReads::default());
}

/// A channel where the account holds a role, and when it was granted (RFC 3339).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleChannel {
    pub login: String,
    pub since: Option<String>,
}

/// Which channels to read, and those left out past the cap.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub join: Vec<String>,
    pub skipped: Vec<String>,
}

/// The account's own channel first, then the newest roles; none that is open as the account
/// (`open`) already, no repeats, at most `cap`.
pub fn plan(own: &str, roles: &[RoleChannel], open: &HashSet<String>, cap: usize) -> Plan {
    let mut by_age: Vec<&RoleChannel> = roles.iter().collect();
    // Newest first; no date last. RFC 3339 in UTC sorts as text.
    by_age.sort_by(|a, b| b.since.cmp(&a.since));
    let mut seen = HashSet::new();
    let mut join: Vec<String> = std::iter::once(own)
        .chain(by_age.iter().map(|r| r.login.as_str()))
        .filter_map(cyex_core::login::normalize)
        .filter(|l| !open.contains(l) && seen.insert(l.clone()))
        .collect();
    let skipped = join.split_off(cap.min(join.len()));
    Plan { join, skipped }
}

/// What to join and what to part to get from `now` to `next`.
fn diff(now: &BTreeSet<String>, next: &[String]) -> (Vec<String>, Vec<String>) {
    let next_set: BTreeSet<&String> = next.iter().collect();
    let join = next.iter().filter(|c| !now.contains(*c)).cloned().collect();
    let part = now.iter().filter(|c| !next_set.contains(c)).cloned().collect();
    (join, part)
}

/// Whose roles to read: the default account's while global mentions have something to find.
fn wanted(active: bool, default_account: Option<&str>) -> Option<String> {
    active.then_some(default_account).flatten().map(str::to_owned)
}

#[derive(Debug, PartialEq, Eq)]
enum Step {
    Keep,
    Stop,
    /// Stops what runs, if anything, and starts for this account.
    Start(String),
}

fn step(running: Option<&str>, wanted: Option<&str>) -> Step {
    match (running, wanted) {
        (a, b) if a == b => Step::Keep,
        (_, None) => Step::Stop,
        (_, Some(w)) => Step::Start(w.to_owned()),
    }
}

/// After global mentions or the accounts changed.
pub fn sync(app: &AppHandle, active: bool) {
    let Some(reads) = app.try_state::<RoleReads>() else { return };
    let accounts = crate::auth::accounts(app);
    let want = wanted(active, accounts.default_account().map(|a| a.user_id.as_str()));
    let mut running = reads.running.lock().unwrap_or_else(|p| p.into_inner());
    match step(running.as_ref().map(|r| r.account.as_str()), want.as_deref()) {
        Step::Keep => {}
        Step::Stop => {
            if let Some(r) = running.take() {
                let _ = r.stop.send(true);
            }
        }
        Step::Start(account) => {
            if let Some(r) = running.take() {
                let _ = r.stop.send(true);
            }
            let (tx, rx) = watch::channel(false);
            *running = Some(Running { account: account.clone(), stop: tx });
            tauri::async_runtime::spawn(run(app.clone(), account, rx));
        }
    }
}

async fn run(app: AppHandle, account: String, mut stop: watch::Receiver<bool>) {
    let creds = Arc::new(WriterCreds(AccountCreds::new(app.clone(), account.clone())));
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (client, task) = Client::with_join_limit(creds, irc::tls(), tx, JOIN_LIMIT);
    tauri::async_runtime::spawn(task);
    let mut joined: BTreeSet<String> = BTreeSet::new();
    let mut roles: Option<(String, Vec<RoleChannel>)> = None;
    let mut last: Option<Plan> = None;
    let mut refresh = tokio::time::interval(REFRESH);
    let mut replan = tokio::time::interval(REPLAN);
    loop {
        let fetch = tokio::select! {
            _ = refresh.tick() => true,
            _ = replan.tick() => roles.is_none(),
            ev = rx.recv() => {
                match ev {
                    Some(Event::Line(raw)) => super::global::irc_line(&app, &raw),
                    Some(Event::Status(LinkStatus::AuthFailed)) => eprintln!("role reads: Twitch refused the sign-in"),
                    Some(_) => {}
                    None => break,
                }
                continue;
            }
            r = stop.changed() => {
                if r.is_err() || *stop.borrow() { break }
                continue;
            }
        };
        if fetch && let Some(r) = read_roles(&app, &account).await {
            roles = Some(r);
        }
        if *stop.borrow() {
            break;
        }
        let Some((own, list)) = &roles else { continue };
        let p = plan(own, list, &super::open_as(&app, &account), MAX_CHANNELS);
        let (join, part) = diff(&joined, &p.join);
        for c in &part {
            client.part(c);
        }
        for c in &join {
            client.join(c);
        }
        if last.as_ref() != Some(&p) {
            if !p.skipped.is_empty() {
                eprintln!(
                    "role reads: reading {} channels, {} past the cap of {MAX_CHANNELS} left out: {}",
                    p.join.len(),
                    p.skipped.len(),
                    p.skipped.join(", ")
                );
            }
            joined = p.join.iter().cloned().collect();
            last = Some(p);
        }
    }
    client.stop();
}

/// The account's login and the channels it moderates. None when Twitch didn't answer (the
/// last list stays); with no sign-in for either read, only the account's own channel.
async fn read_roles(app: &AppHandle, account: &str) -> Option<(String, Vec<RoleChannel>)> {
    let login = crate::auth::accounts(app).get(account)?.login.clone();
    let mut asked = false;
    if let Ok(call) = crate::streammanager::gql_call::gql_call_as(app, account, account.to_owned(), login.clone()) {
        asked = true;
        match community::moderated(&call.channel()).await {
            Ok((items, _)) => {
                let list = items
                    .into_iter()
                    .map(|a| RoleChannel { login: a.channel.login, since: a.since })
                    .collect();
                return Some((login, list));
            }
            Err(e) => eprintln!("role reads: moderated channels (GQL): {e}"),
        }
    }
    if let Some((token, client)) = crate::auth::helix_scoped(app, account, "user:read:moderated_channels") {
        asked = true;
        let http = crate::streammanager::http(app);
        match cyex_twitch::helix::moderated::moderated_channels(&http, crate::auth::helix_auth(&token, &client), account).await {
            Ok(l) => {
                let list = l.into_iter().map(|c| RoleChannel { login: c.broadcaster_login, since: None }).collect();
                return Some((login, list));
            }
            Err(e) => eprintln!("role reads: moderated channels (Helix): {e}"),
        }
    }
    (!asked).then(|| (login, Vec::new()))
}

#[cfg(test)]
mod tests {
    use cyex_chat::irc::Window;

    use super::*;

    fn role(login: &str, since: Option<&str>) -> RoleChannel {
        RoleChannel { login: login.into(), since: since.map(str::to_owned) }
    }

    #[test]
    fn own_channel_first_then_newest_roles() {
        let roles = [role("old", Some("2024-01-01T00:00:00Z")), role("new", Some("2026-11-01T00:00:00Z")), role("undated", None)];
        let p = plan("Me", &roles, &HashSet::new(), MAX_CHANNELS);
        assert_eq!(p.join, ["me", "new", "old", "undated"]);
        assert!(p.skipped.is_empty());
    }

    #[test]
    fn the_cap_leaves_the_oldest_out() {
        let roles: Vec<RoleChannel> = (0..120).map(|i| role(&format!("c{i:03}"), Some(&format!("2026-01-01T00:00:{:02}Z", i % 60)))).collect();
        let p = plan("me", &roles, &HashSet::new(), MAX_CHANNELS);
        assert_eq!(p.join.len(), MAX_CHANNELS);
        assert_eq!(p.join[0], "me");
        assert_eq!(p.skipped.len(), 121 - MAX_CHANNELS);
    }

    #[test]
    fn open_chats_repeats_and_bad_logins_are_left_out() {
        let roles = [role("Open", None), role("twice", None), role("TWICE", None), role("no spaces", None), role("me", None)];
        let open: HashSet<String> = ["open".to_owned()].into();
        assert_eq!(plan("me", &roles, &open, MAX_CHANNELS).join, ["me", "twice"]);
    }

    #[test]
    fn joins_stay_under_twitchs_limit_with_the_accounts_chats() {
        const _: () = assert!(JOIN_LIMIT + Window::JOIN_LIMIT <= 20);
        let now: BTreeSet<String> = ["a", "b"].map(str::to_owned).into();
        let (join, part) = diff(&now, &["b".to_owned(), "c".to_owned()]);
        assert_eq!((join, part), (vec!["c".to_owned()], vec!["a".to_owned()]));
    }

    #[test]
    fn runs_only_while_on_and_signed_in() {
        assert_eq!(wanted(true, Some("1")).as_deref(), Some("1"));
        assert_eq!(wanted(false, Some("1")), None, "global mentions off");
        assert_eq!(wanted(true, None), None, "signed out");
        assert_eq!(step(None, Some("1")), Step::Start("1".into()));
        assert_eq!(step(Some("1"), Some("1")), Step::Keep);
        assert_eq!(step(Some("1"), None), Step::Stop);
        assert_eq!(step(Some("1"), Some("2")), Step::Start("2".into()), "default account changed");
        assert_eq!(step(None, None), Step::Keep);
    }
}
