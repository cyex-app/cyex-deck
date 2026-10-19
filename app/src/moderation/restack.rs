//! V107-M7 A ban or timeout where the last one counts: when Twitch refuses because one is
//! in place, lift the timeout and apply the new one (`cyex_moderation::restack` decides).
//! A timeout never undoes a permanent ban. Used by `/ban`, `/timeout`, the quick timeout
//! buttons (they run `/timeout`) and mass actions.

use std::time::Duration;

use cyex_moderation::restack::{self, Held, Plan, Refusal};
use cyex_twitch::gql;
use cyex_twitch::helix::moderation as hm;
use tauri::AppHandle;

use super::{Actor, codes, feeds, http};

/// Most sends of one ban: the first, a retry after a busy answer, a lift and the replacement.
const TRIES: usize = 4;

/// How Twitch said no, owned.
enum Refused {
    Helix { status: u16, message: String },
    Gql(String),
    Other(String),
}

impl Refused {
    fn refusal(&self) -> Option<Refusal<'_>> {
        match self {
            Refused::Helix { status, message } => Some(Refusal::Helix { status: *status, message }),
            Refused::Gql(code) => Some(Refusal::Gql(code)),
            Refused::Other(_) => None,
        }
    }

    fn text(&self, who: &str) -> String {
        match self {
            Refused::Helix { status, message } => codes::helix(*status, message),
            Refused::Gql(code) => codes::gql(code, who),
            Refused::Other(e) => e.clone(),
        }
    }
}

fn helix_refused(e: &cyex_twitch::Error) -> Refused {
    match e {
        cyex_twitch::Error::Api { status, message } => Refused::Helix { status: *status, message: message.clone() },
        other => Refused::Other(other.to_string()),
    }
}

/// How this account bans: Helix by id, or GQL by login.
enum Path<'a> {
    Helix(cyex_twitch::helix::Auth<'a>, String),
    Gql(gql::Auth<'a>),
}

/// Bans `who`, or times them out for `secs`. `id` is their user id when known.
pub async fn ban(app: &AppHandle, actor: &Actor, who: &str, id: Option<&str>, secs: Option<u32>, reason: &str) -> Result<(), String> {
    let path = match actor.helix_scoped(app, "moderator:manage:banned_users") {
        Some(h) => Path::Helix(h, match id {
            Some(i) => i.to_owned(),
            None => user_id(app, who).await?,
        }),
        None => Path::Gql(actor.gql().ok_or_else(no_token)?),
    };
    let mut last = None;
    for _ in 0..TRIES {
        let Err(r) = once(app, actor, &path, who, secs, reason).await else { return Ok(()) };
        let Some(refusal) = r.refusal() else { return Err(r.text(who)) };
        let mut plan = restack::plan(secs.is_some(), refusal, None);
        if plan == Plan::LookUp {
            let held = match &path {
                Path::Gql(g) => held(app, *g, actor, who, id).await,
                Path::Helix(..) => None,
            };
            plan = match held {
                Some(h) => restack::plan(secs.is_some(), refusal, Some(h)),
                None => Plan::Fail,
            };
        }
        eprintln!("restack: {who} refused ({}), {plan:?}", r.text(who));
        match plan {
            Plan::Retry => tokio::time::sleep(Duration::from_millis(400)).await,
            Plan::Replace => lift(app, actor, &path, who).await?,
            Plan::KeepBan => return Err(restack::keep_ban_line(who)),
            Plan::Fail | Plan::LookUp => return Err(r.text(who)),
        }
        last = Some(r);
    }
    Err(last.map_or_else(|| "Twitch refused it.".into(), |r| r.text(who)))
}

/// One send of the ban or timeout.
async fn once(app: &AppHandle, actor: &Actor, path: &Path<'_>, who: &str, secs: Option<u32>, reason: &str) -> Result<(), Refused> {
    match path {
        Path::Helix(h, id) => {
            let (res, _) = hm::ban(&http(app), *h, actor.room(), id, secs, reason).await;
            res.map_err(|e| helix_refused(&e))
        }
        Path::Gql(g) => {
            let expires = secs.map(|s| format!("{s}s"));
            match gql::moderation::ban(&http(app), *g, &actor.channel_id, who, expires.as_deref(), reason).await {
                Ok(None) => Ok(()),
                Ok(Some(code)) => Err(Refused::Gql(code)),
                Err(e) => Err(Refused::Other(e.to_string())),
            }
        }
    }
}

/// Lifts the current timeout so the new one can go on. Its echo adds no chat line.
async fn lift(app: &AppHandle, actor: &Actor, path: &Path<'_>, who: &str) -> Result<(), String> {
    feeds::own_lift(app, &actor.channel_id, who, true);
    let res = match path {
        Path::Helix(h, id) => hm::unban(&http(app), *h, actor.room(), id).await.0.map_err(|e| helix_refused(&e).text(who)),
        Path::Gql(g) => match gql::moderation::unban(&http(app), *g, &actor.channel_id, who).await {
            Ok(None) => Ok(()),
            Ok(Some(code)) => Err(codes::gql(&code, who)),
            Err(e) => Err(e.to_string()),
        },
    };
    if res.is_err() {
        feeds::own_lift(app, &actor.channel_id, who, false);
    }
    res
}

/// What `who` has now, from Twitch's ban status; `None` when it can't be read.
async fn held(app: &AppHandle, g: gql::Auth<'_>, actor: &Actor, who: &str, id: Option<&str>) -> Option<Held> {
    let id = match id {
        Some(i) => i.to_owned(),
        None => user_id(app, who).await.ok()?,
    };
    let status = gql::modlogs::ban_status(&http(app), g, &actor.channel_id, &id).await.ok()?;
    Some(match status {
        None => Held::Nothing,
        Some(s) if s.until.is_none() => Held::Ban,
        Some(_) => Held::Timeout,
    })
}

async fn user_id(app: &AppHandle, login: &str) -> Result<String, String> {
    gql::profile(&http(app), Some(login), None)
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.id)
        .ok_or_else(|| format!("No user called {login}."))
}

fn no_token() -> String {
    "This account can’t moderate. Sign in again in Settings → Accounts.".into()
}
