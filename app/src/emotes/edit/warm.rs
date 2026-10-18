//! V106-31/32 Reads and keeps what an account can edit.

use std::time::{Duration, Instant};

use cyex_emotes::seventv::account::{Client, Error};
use tauri::AppHandle;

use super::reach::{self, Reach};
use super::{Reachability, gate, gate_state, lock, state};
use crate::cosmetics;

/// How long a read reach is kept.
const KEEP: Duration = Duration::from_secs(5 * 60);

/// K12 After a failed read, how long the menus wait before reading again.
const RETRY: Duration = Duration::from_secs(30);

/// K12 Whether the menus may read again after a failure at `failed`.
pub(super) fn retry_due(failed: Option<Instant>, now: Instant) -> bool {
    failed.is_none_or(|at| now.duration_since(at) >= RETRY)
}

/// The cached answer, if still fresh.
fn cached(app: &AppHandle, account: &str) -> Option<Reachability> {
    let reach = lock(&state(app).reach);
    let (at, r) = reach.get(account)?;
    (at.elapsed() < KEEP).then_some(if r.is_some() {
        Reachability::Ready
    } else {
        Reachability::NoAccount
    })
}

/// Reads what the account can edit, unless a fresh copy is kept (or `fresh`).
pub async fn warm(app: &AppHandle, account: &str, fresh: bool) -> Reachability {
    if let Some(s) = gate_state(app, account) {
        return s;
    }
    if !fresh && let Some(s) = cached(app, account) {
        return s;
    }
    let turn = gate(&state(app).warming, account);
    let _turn = turn.lock().await;
    if !fresh && let Some(s) = cached(app, account) {
        return s;
    }
    match read(&cosmetics::api(app), account).await {
        Ok(r) => {
            let st = state(app);
            lock(&st.down).remove(account);
            let now = if r.is_some() {
                Reachability::Ready
            } else {
                Reachability::NoAccount
            };
            lock(&st.reach).insert(account.to_owned(), (Instant::now(), r));
            now
        }
        Err(e) => {
            eprintln!("7tv edit: {account}: {e}");
            lock(&state(app).down).insert(account.to_owned(), Instant::now());
            Reachability::Down
        }
    }
}

async fn read(api: &Client, account: &str) -> Result<Option<Reach>, Error> {
    let Some(me) = api.reach(account).await? else {
        return Ok(None);
    };
    let ids = reach::set_ids(&me);
    let sets = if ids.is_empty() {
        Vec::new()
    } else {
        api.edit_sets(&ids).await?
    };
    Ok(Some(reach::build(me, sets)))
}

/// Where things stand without a network call; a missing or stale copy starts a warm,
/// at most one per `RETRY` after a failure (K12).
pub(super) fn reachability(app: &AppHandle, account: &str) -> Reachability {
    if let Some(s) = gate_state(app, account).or_else(|| cached(app, account)) {
        return s;
    }
    let failed = lock(&state(app).down).get(account).copied();
    if !retry_due(failed, Instant::now()) {
        return Reachability::Down;
    }
    let (app, owned) = (app.clone(), account.to_owned());
    tauri::async_runtime::spawn(async move {
        warm(&app, &owned, false).await;
    });
    if failed.is_some() {
        Reachability::Down
    } else {
        Reachability::Loading
    }
}
