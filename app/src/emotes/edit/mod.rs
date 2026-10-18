//! V106-31/32 7TV set editing: per account, the channels it can edit (kept 5 min, updated by
//! our edits and 7TV's events), one edit at a time per set, our edits shown in loaded chats at
//! once with their C10 line (C1). Nothing calls 7TV before the account's 7TV consent and a
//! stored session; a test run refuses edits unless the client points at the loopback mock.

mod ask;
mod echo;
mod here;
mod ops;
mod options;
mod reach;
mod send;
#[cfg(test)]
mod tests;
mod warm;

pub use ask::{name_check, options};
#[cfg(test)]
pub(crate) use echo::Echoes;
pub use here::{emote_facts, here_names, here_target};
pub use ops::{Blocked, EditDone, EditFail, EditOp, EditOptions, NameCheck, Reachability};
pub use reach::Target;
pub(crate) use reach::slot_of;
pub use send::run;
pub use warm::warm;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use cyex_emotes::seventv::events::SetChange;
use tauri::{AppHandle, Manager};

use crate::auth::{self, Auth};
use crate::cosmetics;
use reach::Reach;

type Gate = Arc<tokio::sync::Mutex<()>>;

#[derive(Default)]
pub struct Edits {
    /// By account (Twitch id): when read, and what it can edit. `None` = no 7TV account.
    /// Edited in place under this lock; never held across an await.
    reach: Mutex<HashMap<String, (Instant, Option<Reach>)>>,
    /// One read per account at a time.
    warming: Mutex<HashMap<String, Gate>>,
    /// One edit per set at a time.
    gates: Mutex<HashMap<String, Gate>>,
    /// K12 Last read failed at: `Down`, no new read from the menus for `RETRY`.
    down: Mutex<HashMap<String, Instant>>,
    /// C1 Our edits that 7TV's event must not print again.
    echoes: Mutex<echo::Echoes>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn gate(map: &Mutex<HashMap<String, Gate>>, key: &str) -> Gate {
    lock(map).entry(key.to_owned()).or_default().clone()
}

/// After `cosmetics::init`.
pub fn init(app: &AppHandle) {
    app.manage(Edits::default());
}

fn state(app: &AppHandle) -> &Edits {
    app.state::<Edits>().inner()
}

/// No 7TV call without consent and a live session.
fn gate_state(app: &AppHandle, account: &str) -> Option<Reachability> {
    let tokens = &app.state::<Auth>().seventv;
    standing(
        cosmetics::has_consent(app, account),
        tokens.get(account).as_ref(),
        tokens.rejected(account),
        auth::unix_now(),
    )
}

/// Why no edit can go out, or the live session. A session 7TV rejected is gone but still
/// reads "Sign in again" (not "signed out") until the next sign-in.
fn standing(
    consent: bool,
    token: Option<&cyex_auth::SevenTvToken>,
    rejected: bool,
    now: i64,
) -> Option<Reachability> {
    if !consent {
        return Some(Reachability::NoConsent);
    }
    match token {
        None if rejected => Some(Reachability::SignInAgain),
        None => Some(Reachability::SignedOut),
        Some(t) if t.expired(now) => Some(Reachability::SignInAgain),
        Some(_) => None,
    }
}

/// Consent and a live session, or why not.
fn session(app: &AppHandle, account: &str) -> Result<cyex_auth::SevenTvToken, EditFail> {
    let tokens = &app.state::<Auth>().seventv;
    let token = tokens.get(account);
    let why = standing(
        cosmetics::has_consent(app, account),
        token.as_ref(),
        tokens.rejected(account),
        auth::unix_now(),
    );
    match why {
        None => token.ok_or(EditFail::SignedOut),
        Some(Reachability::NoConsent) => Err(EditFail::NoConsent),
        Some(Reachability::SignInAgain) => Err(EditFail::SignInAgain),
        Some(_) => Err(EditFail::SignedOut),
    }
}

/// 7TV's event: every cached target of that set follows it. `c.removed` without our own
/// removals; those are `ours`, exact only (`unseen`).
pub fn on_set_changed(app: &AppHandle, c: &SetChange, ours: &[(String, String)]) {
    let Some(st) = app.try_state::<Edits>() else {
        return;
    };
    let mut reach = lock(&st.reach);
    for t in reach
        .values_mut()
        .filter_map(|(_, r)| r.as_mut())
        .flat_map(|r| r.targets.iter_mut())
        .filter(|t| t.set_id == c.set_id)
    {
        t.apply_change(c, ours);
    }
}

/// C1 What of 7TV's event still needs a line: `c` minus our own edits; and our removals in it,
/// which apply exactly (V105-04: never the id's other name).
pub fn unseen(app: &AppHandle, c: &SetChange) -> (SetChange, Vec<(String, String)>) {
    match app.try_state::<Edits>() {
        Some(st) => lock(&st.echoes).filter(c, Instant::now()),
        None => echo::Echoes::default().filter(c, Instant::now()),
    }
}
