//! 1.36 What the usercard adds to the public profile: sub status (api.ivr.fi, D23), when
//! they followed (only where the chat's account moderates, a Twitch limit), their name
//! history and logs (logs.cyex.app). Logged lines become chat lines with emotes.

use std::sync::Arc;

use cyex_chat::build::{self, Colors, Ctx, Incoming, Me};
use cyex_chat::irc::Line;
use cyex_chat::model::{ChatMessage, Flag};
use cyex_twitch::helix;
use cyex_twitch::logs::{self, Logs, NameChange, SubStatus};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Chat, Identity};

const PAGE: u32 = 50;

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<crate::auth::Auth>().http.clone()
}

pub async fn sub_status(app: &AppHandle, user: &str, channel: &str) -> Result<SubStatus, String> {
    logs::sub_status(&http(app), user, channel)
        .await
        .map_err(|_| "Couldn't get the sub status.".into())
}

pub async fn name_history(app: &AppHandle, user_id: &str) -> Result<Vec<NameChange>, String> {
    logs::name_history(&http(app), user_id)
        .await
        .map_err(|e| e.to_string())
}

/// When they followed, if an account that moderates the channel can ask. `Ok(None)`:
/// not following, or nobody here may ask.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FollowAge {
    /// Some account here moderates the channel, so Twitch answers.
    pub known: bool,
    /// RFC 3339.
    pub since: Option<String>,
}

pub async fn follow_age(app: &AppHandle, channel: &str, user_id: &str) -> Result<FollowAge, String> {
    let (channel_id, mods) = {
        let chat = app.state::<Chat>();
        let inner = chat.lock();
        let mut channel_id = None;
        let mut mods = Vec::new();
        for ((identity, login), e) in &inner.rooms {
            if login != channel {
                continue;
            }
            channel_id = channel_id.or_else(|| e.room.state.channel_id.clone());
            if let Identity::Account(a) = identity
                && e.room.privileged()
            {
                mods.push(a.clone());
            }
        }
        (channel_id, mods)
    };
    let Some(channel_id) = channel_id else {
        return Ok(FollowAge {
            known: false,
            since: None,
        });
    };
    for account in mods {
        let Some((token, client_id)) = crate::auth::helix_scoped(app, &account, "moderator:read:followers")
        else {
            continue;
        };
        let auth = helix::Auth {
            client_id: &client_id,
            token: &token,
        };
        if let Ok(since) = helix::followed_at(&http(app), auth, &channel_id, user_id).await {
            return Ok(FollowAge { known: true, since });
        }
    }
    Ok(FollowAge {
        known: false,
        since: None,
    })
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LogPage {
    #[ts(as = "Vec<ChatMessage>")]
    pub messages: Vec<Arc<ChatMessage>>,
    /// `lines`, `none` (nothing logged) or `optedOut`.
    pub state: String,
    /// More older lines to load.
    pub more: bool,
    /// The browser page for these logs.
    pub url: String,
}

/// A page of someone's logs in a channel, newest first.
pub async fn logs(
    app: &AppHandle,
    channel: &str,
    user_id: &str,
    user: &str,
    offset: u32,
) -> Result<LogPage, String> {
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| e.to_string())?;
    let got = logs::user_logs(&http(app), channel, user_id, PAGE, offset, &now)
        .await
        .map_err(|_| "Couldn't reach the logs.".to_owned())?;
    let url = logs::page_url(channel, user);
    let lines = match got {
        Logs::Lines(l) => l,
        Logs::None => {
            return Ok(LogPage {
                messages: Vec::new(),
                state: "none".into(),
                more: false,
                url,
            });
        }
        Logs::OptedOut => {
            return Ok(LogPage {
                messages: Vec::new(),
                state: "optedOut".into(),
                more: false,
                url,
            });
        }
    };
    let more = lines.len() as u32 == PAGE;
    let emotes = crate::emotes::room(app, None, channel);
    let mut colors = Colors::default();
    let me: Option<Me> = None;
    let now = super::now_ms();
    let messages = lines
        .iter()
        .filter_map(|raw| {
            let line = Line::parse(raw)?;
            let mut ctx = Ctx::new(me.as_ref(), &mut colors, now);
            ctx.emotes = &emotes;
            match build::build(&line, &mut ctx)? {
                Incoming::Message(mut m) => {
                    m.set(Flag::History);
                    Some(Arc::new(m))
                }
                _ => None,
            }
        })
        .collect();
    Ok(LogPage {
        messages,
        state: "lines".into(),
        more,
        url,
    })
}
