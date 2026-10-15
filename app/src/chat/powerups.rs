//! 1.42 Power-ups and 1.43 GIFs for a chat's account. Prices are read fresh before the
//! confirm in the UI; Twitch refuses a price that changed since (`REWARD_COST_MISMATCH`).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cyex_twitch::giphy::{self, Gif};
use cyex_twitch::gql::gifs::{GifAccess, GifSent};
use cyex_twitch::gql::powerups::{self, EFFECTS, PowerUps};
use cyex_twitch::gql::{self, gifs};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Chat, Identity};

/// GIF access is asked again after this long.
const ACCESS_TTL: Duration = Duration::from_secs(300);

/// What a chat's account can use, with the GQL token to use it.
struct Ctx {
    channel_id: String,
    login: String,
    account: Option<String>,
    token: Option<(String, String)>,
}

impl Ctx {
    fn auth(&self) -> gql::Auth<'_> {
        self.token.as_ref().map_or(gql::Auth::PUBLIC, |(t, c)| gql::Auth {
            client_id: c,
            token: Some(t),
        })
    }

    fn signed_in(&self) -> Result<gql::Auth<'_>, String> {
        if self.token.is_none() {
            return Err("This needs the GQL sign-in (Settings → Accounts).".into());
        }
        Ok(self.auth())
    }
}

fn ctx(app: &AppHandle, view: u32) -> Result<Ctx, String> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    let key = inner
        .views
        .get(&view)
        .map(|v| v.0.clone())
        .ok_or("This chat is closed.")?;
    let channel_id = inner
        .rooms
        .get(&key)
        .and_then(|e| e.room.state.channel_id.clone())
        .ok_or("Not connected yet.")?;
    let account = match &key.0 {
        Identity::Account(a) => Some(a.clone()),
        Identity::Anon => None,
    };
    drop(inner);
    let token = account.as_deref().and_then(|a| crate::auth::gql_token(app, a));
    Ok(Ctx {
        channel_id,
        login: key.1,
        account,
        token,
    })
}

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<crate::auth::Auth>().http.clone()
}

pub async fn list(app: &AppHandle, view: u32) -> Result<PowerUps, String> {
    let c = ctx(app, view)?;
    powerups::power_ups(&http(app), c.auth(), &c.login)
        .await
        .map_err(|e| e.to_string())
}

/// What to buy.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum PowerUpBuy {
    Effect { effect: String, message: String },
    Gigantify { emote_id: String, message: String },
    Celebration { emote_id: String },
    Reward { id: String, text: String },
}

/// Spends `bits` (the price the user agreed to).
pub async fn buy(app: &AppHandle, view: u32, what: PowerUpBuy, bits: u32) -> Result<(), String> {
    let c = ctx(app, view)?;
    let auth = c.signed_in()?;
    let http = http(app);
    let code = match &what {
        PowerUpBuy::Effect { effect, message } => {
            if !EFFECTS.contains(&effect.as_str()) {
                return Err("Unknown effect.".into());
            }
            powerups::send_effect(&http, auth, &c.channel_id, message, effect, bits).await
        }
        PowerUpBuy::Gigantify { emote_id, message } => {
            powerups::send_gigantified(&http, auth, &c.channel_id, emote_id, message, bits).await
        }
        PowerUpBuy::Celebration { emote_id } => {
            powerups::send_celebration(&http, auth, &c.channel_id, emote_id, bits).await
        }
        PowerUpBuy::Reward { id, text } => {
            let all = powerups::power_ups(&http, auth, &c.login)
                .await
                .map_err(|e| e.to_string())?;
            let reward = all
                .custom
                .into_iter()
                .find(|r| r.id == *id)
                .ok_or("That reward is gone.")?;
            if reward.bits != bits {
                return Err("The price changed. Try again.".into());
            }
            powerups::redeem_bits_reward(&http, auth, &c.channel_id, &reward, Some(text)).await
        }
    }
    .map_err(|e| e.to_string())?;
    match code {
        None => Ok(()),
        Some(code) => Err(power_up_error(&code)),
    }
}

fn power_up_error(code: &str) -> String {
    match code {
        "REWARD_COST_MISMATCH" => "The price changed. Try again.".into(),
        "INSUFFICIENT_POINTS" => "Not enough Bits.".into(),
        "REWARD_DISABLED" | "REWARD_FORBIDDEN" => "This power-up is off here.".into(),
        "REWARD_GLOBAL_COOLDOWN" => "It’s on cooldown. Try again soon.".into(),
        "REWARD_MAX_PER_STREAM" | "REWARD_MAX_PER_USER_PER_STREAM" => {
            "That’s the limit for this stream.".into()
        }
        "INVALID_EMOTE" => "That emote can’t be used.".into(),
        "STREAM_NOT_LIVE" => "Only while the stream is live.".into(),
        "SLOW_MODE" | "FOLLOWERS_ONLY" | "FOLLOWERS_ONLY_ZERO" | "SUBS_ONLY" | "EMOTE_ONLY" | "R9K_MODE" => {
            "The chat’s mode doesn’t allow it right now.".into()
        }
        "USER_TIMED_OUT" | "USER_BANNED" => "You can’t chat here right now.".into(),
        "MSG_DUPLICATE" => "Same message as before. Change it a little.".into(),
        other => format!("Twitch said no ({other})."),
    }
}

#[derive(Default)]
pub struct GifState {
    /// (account, channel id) → access and when it was asked.
    access: Mutex<HashMap<(String, String), (GifAccess, Instant)>>,
}

pub fn init(app: &AppHandle) {
    app.manage(GifState::default());
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifInfo {
    pub can_send: bool,
    pub reason: Option<String>,
}

async fn access(app: &AppHandle, c: &Ctx) -> Result<GifAccess, String> {
    let account = c.account.clone().ok_or("Sign in to send GIFs.")?;
    let k = (account, c.channel_id.clone());
    if let Some((a, at)) = app
        .state::<GifState>()
        .access
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&k)
        && at.elapsed() < ACCESS_TTL
    {
        return Ok(a.clone());
    }
    let a = gifs::gif_access(&http(app), c.signed_in()?, &c.channel_id)
        .await
        .map_err(|e| e.to_string())?;
    app.state::<GifState>()
        .access
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(k, (a.clone(), Instant::now()));
    Ok(a)
}

pub async fn gif_info(app: &AppHandle, view: u32) -> Result<GifInfo, String> {
    let c = ctx(app, view)?;
    if c.token.is_none() {
        return Ok(GifInfo {
            can_send: false,
            reason: Some("GIFs need the GQL sign-in.".into()),
        });
    }
    let a = access(app, &c).await?;
    Ok(GifInfo {
        can_send: a.can_send,
        reason: a.reason.map(str::to_owned),
    })
}

pub async fn gif_search(app: &AppHandle, view: u32, query: &str, offset: u32) -> Result<Vec<Gif>, String> {
    let c = ctx(app, view)?;
    let a = access(app, &c).await?;
    let key = a.api_key.as_deref().ok_or("GIFs aren’t available here.")?;
    giphy::search(&http(app), key, query, offset, a.rating)
        .await
        .map_err(|e| e.to_string())
}

/// `url` must be the GIPHY address the search gave.
pub async fn gif_send(
    app: &AppHandle,
    view: u32,
    id: &str,
    url: &str,
    search: &str,
) -> Result<GifSent, String> {
    let giphy = url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| h == "giphy.com" || h.ends_with(".giphy.com"))
        })
        .unwrap_or(false);
    if !giphy {
        return Err("That isn’t a GIPHY GIF.".into());
    }
    let c = ctx(app, view)?;
    let a = access(app, &c).await?;
    if !a.can_send {
        return Err(a.reason.unwrap_or("You can’t send GIFs here.").into());
    }
    gifs::send_gif(&http(app), c.signed_in()?, &c.channel_id, id, url, search)
        .await
        .map_err(|e| e.to_string())
}
