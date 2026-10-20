//! Channel points as a viewer (1.9): the rewards menu and redeeming, with the account's
//! GQL token. The two rewards that pick an emote list what's left to pick (phase 7).

use std::collections::{HashMap, HashSet};

use cyex_twitch::gql::{self, EmoteChoice, EmoteModifier, Points, Redeemed};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

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

/// Rewards, plus the balance when the account has a GQL token. `fresh` reads them as
/// they are now; otherwise as joining loaded them (V101-13: the icon on open).
pub async fn points(app: &AppHandle, view: u32, fresh: bool) -> Result<Option<Points>, String> {
    let (login, account) = view_key(app, view)?;
    let Some(c) = catalog::channel(app, &login, fresh).await? else {
        return Ok(None);
    };
    let Some(mut p) = c.points.clone() else { return Ok(None) };
    if let Some((token, client_id)) = account.as_deref().and_then(|id| gql_token(app, id)) {
        let http = app.state::<Auth>().http.clone();
        let auth = crate::auth::gql_auth(&token, &client_id);
        p.balance = gql::balance(&http, auth, &login).await.ok().flatten();
    }
    Ok(Some(p))
}

/// What the two "choose an emote" rewards can still give this account.
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmoteRewardChoices {
    /// Sub emotes to unlock.
    pub unlock: Vec<EmoteChoice>,
    /// Emotes you have, with the versions you don't have yet.
    pub modify: Vec<EmoteChoice>,
    pub modifiers: Vec<EmoteModifier>,
}

pub async fn emote_choices(app: &AppHandle, view: u32) -> Result<EmoteRewardChoices, String> {
    let (login, account) = view_key(app, view)?;
    let account = account.ok_or("Sign in to use channel points.")?;
    let c = catalog::channel(app, &login, false).await?.ok_or("Channel not found.")?;
    let http = app.state::<Auth>().http.clone();
    let (choices, sets) = tokio::join!(
        gql::emote_choices(&http, &login),
        crate::emotes::twitch_sets(app, &account, &c.channel_id, true)
    );
    let choices = choices.map_err(|e| e.to_string())?;
    let sets = sets?;
    let owned: HashMap<&str, &[String]> = sets
        .iter()
        .flat_map(|s| s.emotes.iter())
        .map(|e| (e.id.as_str(), e.modifiers.as_slice()))
        .collect();
    let mut unlock = Vec::new();
    let mut modify = Vec::new();
    for mut e in choices.emotes {
        match owned.get(e.id.as_str()) {
            Some(codes) => {
                let have: HashSet<String> = codes.iter().map(|c| format!("{}_{c}", e.id)).collect();
                e.modifications.retain(|m| !have.contains(&m.id));
                if !e.modifications.is_empty() {
                    modify.push(e);
                }
            }
            None if e.unlockable => {
                e.modifications.clear();
                unlock.push(e);
            }
            None => {}
        }
    }
    Ok(EmoteRewardChoices {
        unlock,
        modify,
        modifiers: choices.modifiers,
    })
}

/// Redeems a reward. Returns a short line for the UI, e.g. the unlocked emote. `emote`
/// is the pick for the two emote rewards.
pub async fn redeem(
    app: &AppHandle,
    view: u32,
    reward_id: &str,
    text: Option<String>,
    emote: Option<String>,
) -> Result<String, String> {
    let (login, account) = view_key(app, view)?;
    let account = account.ok_or("Sign in to use channel points.")?;
    let (token, client_id) =
        gql_token(app, &account).ok_or("Channel points need a GQL sign-in. Add it in Settings → Accounts.")?;
    let c = catalog::channel(app, &login, true).await?.ok_or("Channel not found.")?;
    let points = c.points.as_ref().ok_or("This channel has no channel points.")?;
    let http = app.state::<Auth>().http.clone();
    let auth = crate::auth::gql_auth(&token, &client_id);
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
            "CHOSEN_SUB_EMOTE_UNLOCK" => {
                let emote = emote.as_deref().ok_or("Pick an emote first.")?;
                gql::unlock_chosen_emote(&http, auth, id, r.cost, emote).await
            }
            "CHOSEN_MODIFIED_SUB_EMOTE_UNLOCK" => {
                let emote = emote.as_deref().ok_or("Pick an emote first.")?;
                gql::unlock_modified_emote(&http, auth, id, r.cost, emote).await
            }
            _ => return Err("This reward isn’t supported yet.".into()),
        }
    } else {
        return Err("That reward is gone.".into());
    }
    .map_err(|e| e.to_string())?;

    if result.error.is_none() && points.automatic.iter().any(|r| r.id == reward_id) {
        // A new emote: the menu and our own lines should know it.
        crate::emotes::refresh_mine(app, &account, id);
    }
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
        "USER_BANNED" => "You’re banned here.".into(),
        "USER_TIMED_OUT" => "You’re timed out.".into(),
        "FORBIDDEN" | "REWARD_FORBIDDEN" => "You can’t redeem this one.".into(),
        "DUPLICATE_TRANSACTION" | "TRANSACTION_IN_PROGRESS" | "TRANSACTION_ALREADY_COMMITTED" => "Already redeeming.".into(),
        "MSG_DUPLICATE" => "Same as your last message.".into(),
        "MESSAGE_IS_COMMAND" => "The message can’t be a command.".into(),
        "EMOTE_ALREADY_ENTITLED" | "NO_EMOTES_AVAILABLE" => "No emotes left to unlock.".into(),
        other => format!("Didn’t work ({other})."),
    }
}
