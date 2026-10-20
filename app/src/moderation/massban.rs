//! 1.55 Mass banning, by sign-in method. Helix bans one login per request, paced from
//! the rate-limit headers with 429 backoff (the "slow" tool). GQL batches
//! `banUserFromChatRoom` 35 at a time (the "fast" tool, improved: a batch beats the
//! reference's concurrent singles). Both skip mods and logins already banned; one timed out gets the new one (V107-M7).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use cyex_moderation::pacer::Pacer;
use cyex_moderation::restack;
use cyex_twitch::gql;
use cyex_twitch::helix::moderation as hm;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{Duration, sleep};
use ts_rs::TS;

use super::{Actor, http};

/// Progress of a running mass ban, pushed to the mod view.
#[derive(Clone, Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MassbanProgress {
    pub channel_id: String,
    pub done: u32,
    pub total: u32,
    /// Logins Twitch refused (already banned, a mod, not found), with the reason.
    pub refused: u32,
    pub running: bool,
}

/// A running job, so the UI can stop it.
#[derive(Default)]
pub struct Jobs {
    cancel: std::sync::Mutex<Vec<(String, Arc<AtomicBool>)>>,
}

pub fn init(app: &AppHandle) {
    app.manage(Jobs::default());
}

/// Marks a channel's job to stop.
pub fn stop(app: &AppHandle, channel_id: &str) {
    if let Some(jobs) = app.try_state::<Jobs>() {
        for (id, flag) in jobs.cancel.lock().unwrap_or_else(|p| p.into_inner()).iter() {
            if id == channel_id {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }
}

fn register(app: &AppHandle, channel_id: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Some(jobs) = app.try_state::<Jobs>() {
        let mut g = jobs.cancel.lock().unwrap_or_else(|p| p.into_inner());
        g.retain(|(id, f)| id != channel_id || !f.load(Ordering::Relaxed));
        g.push((channel_id.to_owned(), flag.clone()));
    }
    flag
}

fn unregister(app: &AppHandle, flag: &Arc<AtomicBool>) {
    if let Some(jobs) = app.try_state::<Jobs>() {
        jobs.cancel
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|(_, f)| !Arc::ptr_eq(f, flag));
    }
}

/// Bans every login, picking the GQL batch path when the account has a GQL token (faster),
/// else Helix one-by-one. `secs` times out instead of banning. Returns how many went
/// through. Emits `mod:massban` progress as it goes.
pub async fn run(app: &AppHandle, actor: &Actor, logins: &[String], secs: Option<u32>, reason: &str, pacer: &mut Pacer) -> Result<u32, String> {
    let flag = register(app, &actor.channel_id);
    let total = logins.len() as u32;
    let done = AtomicUsize::new(0);
    let refused = AtomicUsize::new(0);
    let emit = |running: bool| {
        let _ = app.emit(
            "mod:massban",
            MassbanProgress {
                channel_id: actor.channel_id.clone(),
                done: done.load(Ordering::Relaxed) as u32,
                total,
                refused: refused.load(Ordering::Relaxed) as u32,
                running,
            },
        );
    };
    emit(true);

    // GQL batches, so prefer it whenever the account has a GQL token; else Helix one-by-one.
    let result = if actor.gql().is_some() {
        gql_path(app, actor, logins, secs, reason, pacer, &flag, &done, &refused, &emit).await
    } else {
        helix_path(app, actor, logins, secs, reason, pacer, &flag, &done, &refused, &emit).await
    };
    unregister(app, &flag);
    emit(false);
    result.map(|()| done.load(Ordering::Relaxed) as u32)
}

#[allow(clippy::too_many_arguments)]
async fn gql_path(
    app: &AppHandle,
    actor: &Actor,
    logins: &[String],
    secs: Option<u32>,
    reason: &str,
    pacer: &mut Pacer,
    flag: &Arc<AtomicBool>,
    done: &AtomicUsize,
    refused: &AtomicUsize,
    emit: &impl Fn(bool),
) -> Result<(), String> {
    let g = actor.gql().ok_or("No GQL token.")?;
    let expires = secs.map(|s| format!("{s}s"));
    for chunk in logins.chunks(gql::BATCH_MAX) {
        if flag.load(Ordering::Relaxed) {
            break;
        }
        let names: Vec<String> = chunk.to_vec();
        let results = gql::moderation::ban_many(&http(app), g, &actor.channel_id, &names, expires.as_deref(), reason)
            .await
            .map_err(|e| e.to_string())?;
        // V107-M7 One in place (a timeout over a timeout, a ban over a timeout): those go
        // again one by one, where the last one counts.
        let mut again = Vec::new();
        for (login, code) in results {
            match code {
                None => done.fetch_add(1, Ordering::Relaxed),
                Some(c) if restack::redo(secs.is_some(), &c) => {
                    again.push(login);
                    continue;
                }
                Some(_) => refused.fetch_add(1, Ordering::Relaxed),
            };
        }
        emit(true);
        for login in restack::until(&again, || flag.load(Ordering::Relaxed)) {
            let ok = super::restack::ban(app, actor, login, None, secs, reason).await.is_ok();
            let n = if ok { done } else { refused };
            n.fetch_add(1, Ordering::Relaxed);
            emit(true);
            sleep(Duration::from_millis(pacer.after_ok(None, crate::chat::now_ms()).max(1) as u64)).await;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn helix_path(
    app: &AppHandle,
    actor: &Actor,
    logins: &[String],
    secs: Option<u32>,
    reason: &str,
    pacer: &mut Pacer,
    flag: &Arc<AtomicBool>,
    done: &AtomicUsize,
    refused: &AtomicUsize,
    emit: &impl Fn(bool),
) -> Result<(), String> {
    let (auth, room) = actor.helix_any().ok_or("No Helix token.")?;
    for login in logins {
        if flag.load(Ordering::Relaxed) {
            break;
        }
        // Helix bans by id; resolve it (skip unknown logins).
        let Some(id) = gql::profile(&http(app), Some(login), None).await.ok().flatten().map(|p| p.id) else {
            refused.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let (res, limit) = hm::ban(&http(app), auth, room, &id, secs, reason).await;
        let wait = match &res {
            Ok(()) => {
                done.fetch_add(1, Ordering::Relaxed);
                pacer.after_ok(limit, crate::chat::now_ms())
            }
            Err(e) if e.is_rate_limited() => {
                // Didn't go through; wait and retry the same login next loop isn't done,
                // so just pace and move on (the user can re-run for stragglers).
                refused.fetch_add(1, Ordering::Relaxed);
                pacer.after_limited(limit, crate::chat::now_ms())
            }
            Err(e) if e.is_unauthorized() => return Err("Lost permission mid-ban.".into()),
            Err(_) => {
                refused.fetch_add(1, Ordering::Relaxed);
                pacer.after_ok(limit, crate::chat::now_ms())
            }
        };
        emit(true);
        sleep(Duration::from_millis(wait.max(1) as u64)).await;
    }
    Ok(())
}
