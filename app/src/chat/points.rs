//! Channel points as a viewer (1.9): the rewards menu and redeeming, with the account's
//! GQL token.

use cyex_twitch::gql::{self, Points, Redeemed};
use tauri::{AppHandle, Manager};

use super::{Chat, Identity, catalog};
use crate::auth::{Auth, gql_token};

/// What the chat a view shows belongs to: channel login and the reading account.
fn view_key(app: &AppHandle, view: u32) -> Result<(String, Option<String>), String> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    let ((identity, login), _) = inner.views.get(&view).cloned().ok_or("This chat is closed.")?;
    let account = match identity {
        Identity::Account(id) => Some(id),
        Identity::Anon => None,
    };
    Ok((login, account))
}

/// Rewards right now, plus the balance when the account has a GQL token.
pub async fn points(app: &AppHandle, view: u32) -> Result<Option<Points>, String> {
    let (login, account) = view_key(app, view)?;
    let Some(c) = catalog::channel(app, &login, true).await? else {
        return Ok(None);
    };
    let Some(mut p) = c.points.clone() else { return Ok(None) };
    if let Some((token, client_id)) = account.as_deref().and_then(|id| gql_token(app, id)) {
        let http = app.state::<Auth>().http.clone();
        let auth = gql::Auth {
            client_id: &client_id,
            token: Some(&token),
        };
        p.balance = gql::balance(&http, auth, &login).await.ok().flatten();
    }
    Ok(Some(p))
}

/// Redeems a reward. Returns a short line for the UI, e.g. the unlocked emote.
pub async fn redeem(app: &AppHandle, view: u32, reward_id: &str, text: Option<String>) -> Result<String, String> {
    let (login, account) = view_key(app, view)?;
    let account = account.ok_or("Sign in to use channel points.")?;
    let (token, client_id) =
        gql_token(app, &account).ok_or("Channel points need a GQL sign-in. Add it in Settings → Accounts.")?;
    let c = catalog::channel(app, &login, true).await?.ok_or("Channel not found.")?;
    let points = c.points.as_ref().ok_or("This channel has no channel points.")?;
    let http = app.state::<Auth>().http.clone();
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let text = text.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
    let id = &c.channel_id;

    let result: Redeemed = if let Some(r) = points.custom.iter().find(|r| r.id == reward_id) {
        if r.input_required && text.is_none() {
            return Err("This one needs a message.".into());
        }
        gql::redeem_custom(&http, auth, id, r, text.as_deref()).await
    } else if let Some(r) = points.automatic.iter().find(|r| r.id == reward_id) {
        match r.kind.as_str() {
            "SEND_HIGHLIGHTED_MESSAGE" => {
                gql::send_highlighted(&http, auth, id, r.cost, text.as_deref().ok_or("Write the message first.")?).await
            }
            "SINGLE_MESSAGE_BYPASS_SUB_MODE" => {
                gql::send_in_sub_mode(&http, auth, id, r.cost, text.as_deref().ok_or("Write the message first.")?).await
            }
            "RANDOM_SUB_EMOTE_UNLOCK" => gql::unlock_random_emote(&http, auth, id, r.cost).await,
            _ => return Err("This reward isn't supported yet.".into()),
        }
    } else {
        return Err("That reward is gone.".into());
    }
    .map_err(|e| e.to_string())?;

    match result.error {
        Some(code) => Err(error_text(&code)),
        None => Ok(match result.emote {
            Some(emote) => format!("Unlocked {emote}."),
            None => "Redeemed.".into(),
        }),
    }
}

fn error_text(code: &str) -> String {
    match code {
        "NOT_ENOUGH_POINTS" | "INSUFFICIENT_POINTS" => "Not enough points.".into(),
        "PROPERTIES_MISMATCH" | "REWARD_COST_MISMATCH" => "This reward just changed. Try again.".into(),
        "DISABLED" | "REWARD_DISABLED" => "This reward is off right now.".into(),
        "STREAM_NOT_LIVE" => "Only while the stream is live.".into(),
        "MAX_PER_STREAM" | "MAX_PER_USER_PER_STREAM" => "Limit reached for this stream.".into(),
        "GLOBAL_COOLDOWN" => "On cooldown.".into(),
        "USER_BANNED" => "You're banned here.".into(),
        "USER_TIMED_OUT" => "You're timed out.".into(),
        "FORBIDDEN" | "REWARD_FORBIDDEN" => "You can't redeem this one.".into(),
        "DUPLICATE_TRANSACTION" | "TRANSACTION_IN_PROGRESS" | "TRANSACTION_ALREADY_COMMITTED" => "Already redeeming.".into(),
        "MSG_DUPLICATE" => "Same as your last message.".into(),
        "MESSAGE_IS_COMMAND" => "The message can't be a command.".into(),
        "EMOTE_ALREADY_ENTITLED" | "NO_EMOTES_AVAILABLE" => "No emotes left to unlock.".into(),
        other => format!("Didn't work ({other})."),
    }
}
