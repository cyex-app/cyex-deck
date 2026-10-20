//! 1.55 Mass ban, timeout or unban, by sign-in method. One path per run, never both at
//! once (V107-R21): GQL when the account has a GQL sign-in, batched `GQL_BATCH` a request
//! one request a second; else Helix one login per request, paced from the rate-limit
//! headers. Logins are looked up first, 100 per call, so gone ones cost no ban. Throttled
//! answers are sent again with backoff; every login ends in one tally bucket. Twitch
//! removed IRC mod commands in Feb 2023, so IRC can't add capacity.
//! A login timed out gets the new one (V107-M7).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cyex_moderation::mass::{self, Outcome, Tally};
use cyex_moderation::mass_run::{self as run, Fault};
use cyex_moderation::pacer::Pacer;
use cyex_twitch::helix::moderation as hm;
use cyex_twitch::{gql, helix};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{Duration, sleep};
use ts_rs::TS;

use super::{Actor, http};

/// Progress of a running mass action, pushed to the mod view.
#[derive(Clone, Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MassbanProgress {
    pub channel_id: String,
    /// Logins handled so far, whatever the outcome.
    pub done: u32,
    pub total: u32,
    /// Logins that didn't go through (the tally says why).
    pub refused: u32,
    pub running: bool,
    pub unban: bool,
    pub tally: Tally,
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

/// What one run does.
#[derive(Clone, Copy)]
pub struct Order<'a> {
    /// A timeout instead of a ban.
    pub secs: Option<u32>,
    pub reason: &'a str,
    pub unban: bool,
}

/// One run's state: the tally and how to report it.
struct Job<'a> {
    app: &'a AppHandle,
    actor: &'a Actor,
    order: Order<'a>,
    flag: Arc<AtomicBool>,
    total: u32,
    tally: Tally,
}

impl Job<'_> {
    fn emit(&self, running: bool) {
        let _ = self.app.emit(
            "mod:massban",
            MassbanProgress {
                channel_id: self.actor.channel_id.clone(),
                done: self.tally.handled(),
                total: self.total,
                refused: self.tally.refused(),
                running,
                unban: self.order.unban,
                tally: self.tally.clone(),
            },
        );
    }

    fn stopped(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}

/// Bans (or times out, or unbans) every login. Returns how many went through. Emits
/// `mod:massban` progress after every request.
pub async fn run(app: &AppHandle, actor: &Actor, logins: &[String], order: Order<'_>, pacer: &mut Pacer) -> Result<u32, String> {
    let flag = register(app, &actor.channel_id);
    let mut job = Job {
        app,
        actor,
        order,
        flag: flag.clone(),
        total: logins.len() as u32,
        tally: Tally::default(),
    };
    job.emit(true);
    let result = if actor.gql().is_some() {
        gql_path(&mut job, logins).await
    } else {
        helix_path(&mut job, logins, pacer).await
    };
    unregister(app, &flag);
    job.emit(false);
    result.map(|()| job.tally.done)
}

fn wait(ms: i64) -> impl Future<Output = ()> {
    sleep(Duration::from_millis(ms.max(1) as u64))
}

/// Whether a whole request failed in a way worth sending again.
fn transient(e: &cyex_twitch::Error) -> bool {
    e.is_rate_limited() || e.is_server_error() || e.is_network()
}

/// The logins Twitch knows (gone ones go into the tally), with their ids. A lookup that
/// keeps failing sends the rest as they are (GQL) or stops (Helix needs ids).
async fn resolve(job: &mut Job<'_>, logins: &[String], helix_auth: Option<helix::Auth<'_>>) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::with_capacity(logins.len());
    for chunk in logins.chunks(100) {
        let mut n = 0;
        let found = loop {
            let res = match helix_auth {
                Some(a) => {
                    let refs: Vec<&str> = chunk.iter().map(String::as_str).collect();
                    helix::users_by_login(&http(job.app), a, &refs)
                        .await
                        .map(|us| us.into_iter().map(|u| (u.login, u.id)).collect::<Vec<_>>())
                }
                None => gql::ids_by_login(&http(job.app), chunk).await,
            };
            match res {
                Ok(f) => break Some(f),
                Err(e) if e.is_unauthorized() => return Err("Lost permission mid-ban.".into()),
                Err(e) if transient(&e) && n + 1 < mass::TRIES => {
                    n += 1;
                    wait(mass::backoff_ms(n, None)).await;
                }
                Err(e) if helix_auth.is_some() => return Err(e.to_string()),
                Err(_) => break None,
            }
        };
        let Some(found) = found else {
            // GQL bans by login: send them unchecked.
            out.extend(chunk.iter().map(|l| (l.clone(), String::new())));
            continue;
        };
        let ids: HashMap<String, String> = found.into_iter().map(|(l, i)| (l.to_ascii_lowercase(), i)).collect();
        for login in chunk {
            match ids.get(&login.to_ascii_lowercase()) {
                Some(id) => out.push((login.clone(), id.clone())),
                None => job.tally.add(Outcome::Gone, None),
            }
        }
    }
    job.emit(true);
    Ok(out)
}

/// The app's side of the loops: real waits, the Stop button, progress to the panel.
struct Live<'a, 'b> {
    job: &'b mut Job<'a>,
    gql: Option<gql::Auth<'b>>,
    helix: Option<(helix::Auth<'b>, helix::Room<'b>)>,
    expires: Option<String>,
    pacer: Option<&'b mut Pacer>,
}

impl run::Io for Live<'_, '_> {
    async fn wait(&mut self, ms: i64) {
        wait(ms).await;
    }

    fn stopped(&self) -> bool {
        self.job.stopped()
    }

    fn progress(&mut self, tally: &Tally) {
        self.job.tally = tally.clone();
        self.job.emit(true);
    }
}

fn fault(e: &cyex_twitch::Error) -> Fault {
    if e.is_unauthorized() {
        Fault::Unauthorized
    } else if e.is_rate_limited() {
        Fault::Throttled
    } else if transient(e) {
        Fault::Busy
    } else {
        Fault::Fatal(e.to_string())
    }
}

impl run::Batches for Live<'_, '_> {
    async fn send(&mut self, logins: &[String]) -> Result<Vec<(String, Option<String>)>, Fault> {
        let (app, cid) = (self.job.app, &self.job.actor.channel_id);
        let g = self.gql.ok_or(Fault::Unauthorized)?;
        let res = if self.job.order.unban {
            gql::moderation::unban_many(&http(app), g, cid, logins).await
        } else {
            gql::moderation::ban_many(&http(app), g, cid, logins, self.expires.as_deref(), self.job.order.reason).await
        };
        res.map_err(|e| fault(&e))
    }
}

impl run::Singles for Live<'_, '_> {
    async fn send(&mut self, id: &str) -> run::Answer {
        let app = self.job.app;
        let Some((auth, room)) = self.helix else { return (Err((401, String::new())), 0) };
        let (res, limit) = if self.job.order.unban {
            hm::unban(&http(app), auth, room, id).await
        } else {
            hm::ban(&http(app), auth, room, id, self.job.order.secs, self.job.order.reason).await
        };
        let now = crate::chat::now_ms();
        let pacer = self.pacer.as_deref_mut().expect("Helix runs with a pacer");
        match res {
            Ok(()) => (Ok(()), pacer.after_ok(limit, now)),
            Err(e) if e.is_rate_limited() => (Err((429, String::new())), pacer.after_limited(limit, now)),
            Err(cyex_twitch::Error::Api { status, message }) => (Err((status, message)), pacer.after_ok(limit, now)),
            Err(e) if e.is_network() => (Err((0, String::new())), pacer.after_ok(limit, now)),
            Err(e) => (Err((400, e.to_string())), pacer.after_ok(limit, now)),
        }
    }
}

async fn gql_path(job: &mut Job<'_>, logins: &[String]) -> Result<(), String> {
    let actor = job.actor;
    let g = actor.gql().ok_or("No GQL token.")?;
    let names: Vec<String> = resolve(job, logins, None).await?.into_iter().map(|(l, _)| l).collect();
    let order = job.order;
    let mut tally = std::mem::take(&mut job.tally);
    let again = {
        let mut live = Live { job, gql: Some(g), helix: None, expires: order.secs.map(|s| format!("{s}s")), pacer: None };
        run::batches(&mut live, &names, mass::GQL_BATCH, order.secs.is_some(), order.unban, &mut tally).await?
    };
    job.tally = tally;
    // V107-M7 One in place (a timeout over a timeout, a ban over a timeout): one by
    // one, where the last one counts.
    for login in again {
        if job.stopped() {
            break;
        }
        wait(mass::GQL_GAP_MS).await;
        match super::restack::ban(job.app, actor, &login, None, order.secs, order.reason).await {
            Ok(()) => job.tally.add(Outcome::Done, None),
            Err(t) if t.contains("banned") => job.tally.add(Outcome::Already, None),
            Err(t) => job.tally.add(Outcome::Other, Some(&t)),
        }
        job.emit(true);
    }
    Ok(())
}

async fn helix_path(job: &mut Job<'_>, logins: &[String], pacer: &mut Pacer) -> Result<(), String> {
    let actor = job.actor;
    let (auth, room) = actor.helix_any().ok_or("No Helix token.")?;
    let ids: Vec<String> = resolve(job, logins, Some(auth)).await?.into_iter().map(|(_, id)| id).collect();
    let unban = job.order.unban;
    let mut tally = std::mem::take(&mut job.tally);
    let res = {
        let mut live = Live { job, gql: None, helix: Some((auth, room)), expires: None, pacer: Some(pacer) };
        run::singles(&mut live, &ids, unban, &mut tally).await
    };
    job.tally = tally;
    res
}
