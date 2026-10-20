//! V107-M7 A ban or timeout where the last one counts: when Twitch refuses because one is
//! in place, lift the timeout and apply the new one (`cyex_moderation::restack` decides).
//! A timeout never undoes a permanent ban. Used by `/ban`, `/timeout`, the quick timeout
//! buttons (they run `/timeout`) and mass actions.

use std::time::Duration;

use cyex_moderation::restack::{self, Held, How, Refused};
use cyex_twitch::gql;
use cyex_twitch::helix::moderation as hm;
use tauri::AppHandle;

use super::{Actor, codes, feeds, http};

fn helix_refused(e: &cyex_twitch::Error) -> Refused {
    match e {
        cyex_twitch::Error::Api { status, message } => Refused {
            how: Some(How::Helix { status: *status, message: message.clone() }),
            text: codes::helix(*status, message),
        },
        other => Refused { how: None, text: other.to_string() },
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
    let mut tw = Live { app, actor, path, who, id, secs, reason };
    restack::run(&mut tw, who, secs.is_some()).await
}

/// One ban against Twitch, for `restack::run`.
struct Live<'a> {
    app: &'a AppHandle,
    actor: &'a Actor,
    path: Path<'a>,
    who: &'a str,
    id: Option<&'a str>,
    secs: Option<u32>,
    reason: &'a str,
}

impl restack::Twitch for Live<'_> {
    async fn send(&mut self) -> Result<(), Refused> {
        let (app, actor, who) = (self.app, self.actor, self.who);
        let res = match &self.path {
            Path::Helix(h, id) => hm::ban(&http(app), *h, actor.room(), id, self.secs, self.reason).await.0.map_err(|e| helix_refused(&e)),
            Path::Gql(g) => {
                let expires = self.secs.map(|s| format!("{s}s"));
                match gql::moderation::ban(&http(app), *g, &actor.channel_id, who, expires.as_deref(), self.reason).await {
                    Ok(None) => Ok(()),
                    Ok(Some(code)) => Err(Refused { text: codes::gql(&code, who), how: Some(How::Gql(code)) }),
                    Err(e) => Err(Refused { how: None, text: e.to_string() }),
                }
            }
        };
        if let Err(r) = &res {
            eprintln!("restack: {who} refused ({})", r.text);
        }
        res
    }

    async fn held(&mut self) -> Option<Held> {
        match &self.path {
            Path::Gql(g) => held(self.app, *g, self.actor, self.who, self.id).await,
            Path::Helix(..) => None,
        }
    }

    /// Lifts the current timeout so the new one can go on. Its echo adds no chat line.
    async fn lift(&mut self) -> Result<(), String> {
        let (app, actor, who) = (self.app, self.actor, self.who);
        feeds::own_lift(app, &actor.channel_id, who, true);
        let res = match &self.path {
            Path::Helix(h, id) => hm::unban(&http(app), *h, actor.room(), id).await.0.map_err(|e| helix_refused(&e).text),
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

    fn unmark_lift(&mut self) {
        feeds::own_lift(self.app, &self.actor.channel_id, self.who, false);
    }

    async fn pause(&mut self) {
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
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
