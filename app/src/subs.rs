//! 1.28 Subscriptions without twitch.tv: the account's subs, its standing in a channel,
//! sharing a resub in chat, the free Prime sub and turning renewal off, all through GQL.
//! Buying, gifting, changing tier and payment methods open Twitch's own pages in the
//! app's Twitch window (`windows::twitch`, 3.8).

use cyex_twitch::gql::subs::{self as api, ChannelSub, Subs, pages};
use tauri::AppHandle;

use crate::whispers::GqlCall;

const NO_GQL: &str = "This needs a GQL sign-in for the account (Settings → Accounts).";

fn call(app: &AppHandle, account: &str) -> Result<GqlCall, String> {
    GqlCall::new(app, account).map_err(|_| NO_GQL.to_owned())
}

fn failed(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        return "Couldn’t reach Twitch.".into();
    }
    eprintln!("subs: {e}");
    match e.api_message() {
        Some(m) if m.contains("integrity") => "Twitch wants this done on its own page.".into(),
        _ => "Twitch didn’t take that. Try again in a bit.".into(),
    }
}

pub async fn mine(app: &AppHandle, account: &str) -> Result<Subs, String> {
    let c = call(app, account)?;
    api::mine(&c.http, c.auth()).await.map_err(failed)
}

pub async fn channel(app: &AppHandle, account: &str, login: &str) -> Result<Option<ChannelSub>, String> {
    let c = call(app, account)?;
    api::channel(&c.http, c.auth(), login).await.map_err(failed)
}

pub async fn share_resub(
    app: &AppHandle,
    account: &str,
    login: &str,
    token_id: &str,
    message: &str,
    streak: bool,
) -> Result<(), String> {
    if message.chars().count() > 500 {
        return Err("The message can be 500 characters at most.".into());
    }
    let c = call(app, account)?;
    api::share_resub(&c.http, c.auth(), login, token_id, message.trim(), streak)
        .await
        .map_err(failed)
}

pub async fn prime(app: &AppHandle, account: &str, channel_id: &str) -> Result<(), String> {
    let c = call(app, account)?;
    api::prime(&c.http, c.auth(), account, channel_id)
        .await
        .map_err(|e| match e.api_message() {
            Some("NO_CREDIT" | "NO_SUB_CREDIT") => "Your Prime sub is already in use this month.".into(),
            Some("ALREADY_SUBSCRIBED" | "EXISTING_SUBSCRIPTION") => "You’re already subscribed there.".into(),
            _ => failed(e),
        })
}

pub async fn stop_renewal(app: &AppHandle, account: &str, sub_id: &str) -> Result<(), String> {
    let c = call(app, account)?;
    api::stop_renewal(&c.http, c.auth(), account, sub_id)
        .await
        .map_err(failed)
}

/// Which Twitch page a sub action opens in the app's Twitch window.
pub fn page(kind: &str, login: &str) -> Option<String> {
    let login_ok = !login.is_empty()
        && login.len() <= 25
        && login.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if matches!(kind, "subscribe" | "gift") && !login_ok {
        return None;
    }
    Some(match kind {
        "wallet" => pages::WALLET.to_owned(),
        "subscriptions" => pages::SUBSCRIPTIONS.to_owned(),
        "subscribe" => pages::subscribe(login),
        "gift" => pages::gift(login),
        _ => return None,
    })
}
