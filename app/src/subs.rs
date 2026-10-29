//! 1.28 Subscriptions without twitch.tv: the account's subs, its standing in a channel,
//! sharing a resub in chat, the free Prime sub and turning renewal off, all through GQL.
//! Buying, gifting, changing tier and payment methods open Twitch's own pages in the
//! app's Twitch window (`windows::twitch`, 3.8).

use cyex_miner::events::{self as miner_events, Event as MinerEvent};
use cyex_twitch::gql::milestones::{self, Milestone};
use cyex_twitch::gql::subs::{self as api, ChannelSub, Subs, pages};
use cyex_twitch::hermes::{Event, Kind};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

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

/// V108-N23 Tells open chats to read the watch streak again.
const STREAK_EVENT: &str = "subs:streak";

/// V108-N23 What the account can share in a channel now, and whether the channel is live (the
/// chat then re-reads every minute as a fallback to the PubSub topic).
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Streak {
    pub milestone: Option<Milestone>,
    pub live: bool,
}

/// V108-N23 A milestone or streak changed in this channel.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreakChanged {
    pub account: String,
    pub channel_id: String,
}

pub async fn streak(app: &AppHandle, account: &str, login: &str) -> Result<Streak, String> {
    let c = call(app, account)?;
    let milestone = milestones::shareable_streak(&c.http, c.auth(), login).await.map_err(failed)?;
    Ok(Streak { milestone, live: crate::tabs::live::is_live(app, login) })
}

/// V108-N23 Only from the user's click on Share.
pub async fn share_streak(app: &AppHandle, account: &str, channel_id: &str, milestone_id: &str) -> Result<(), String> {
    let c = call(app, account)?;
    milestones::share(&c.http, c.auth(), channel_id, milestone_id)
        .await
        .map_err(|e| match e {
            cyex_twitch::Error::Refused(code) => milestones::explain(&code).to_owned(),
            e => failed(e),
        })
}

/// V108-N23 Follows (or stops following) the account's milestones topic while a chat shows
/// the prompt. The topic is counted, so several chats can share it.
pub fn watch_streak(app: &AppHandle, account: &str, on: bool) {
    if on {
        crate::hermes::add_user(app, account, Kind::ViewerMilestones);
    } else {
        crate::hermes::remove_user(app, account, Kind::ViewerMilestones);
    }
}

/// V108-N23 A `viewer-milestones` message: the chat of that channel reads again.
pub fn on_milestones(app: &AppHandle, e: &Event) {
    if let Some((channel_id, _)) = milestones::update_of(&e.payload) {
        changed(app, &e.topic.channel_id, channel_id);
    }
}

/// V108-N23 A `WATCH_STREAK` points gain on the account's points topic: the streak moved.
pub fn on_points(app: &AppHandle, e: &Event) {
    if let Some(MinerEvent::Earned { channel_id, reason, .. }) = miner_events::parse(&e.payload)
        && reason == milestones::WATCH_STREAK
    {
        changed(app, &e.topic.channel_id, channel_id);
    }
}

fn changed(app: &AppHandle, account: &str, channel_id: String) {
    let _ = app.emit(STREAK_EVENT, StreakChanged { account: account.to_owned(), channel_id });
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
        "bits" => pages::BITS.to_owned(),
        "subscribe" => pages::subscribe(login),
        "gift" => pages::gift(login),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_page_needs_no_channel() {
        assert_eq!(page("bits", "").as_deref(), Some("https://www.twitch.tv/bits"));
        assert_eq!(page("bits", "x").as_deref(), Some(pages::BITS));
        assert_eq!(page("nope", ""), None);
        assert_eq!(page("gift", ""), None);
    }
}
