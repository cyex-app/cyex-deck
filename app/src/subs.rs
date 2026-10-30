//! 1.28 Subscriptions without twitch.tv: the account's subs, its standing in a channel,
//! sharing a resub in chat, the free Prime sub and turning renewal off, all through GQL.
//! Buying, gifting, changing tier and payment methods open Twitch's own pages in the
//! app's Twitch window (`windows::twitch`, 3.8).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

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

/// V108-N23 A streak read is kept this long per (account, channel login), so every chat of a
/// channel and each reopen don't each ask Twitch. A change on the topic drops the account's.
const STREAK_AGE: Duration = Duration::from_secs(60);
/// The most reads kept; past it the cache starts over.
const STREAK_KEPT: usize = 64;

#[derive(Default)]
struct StreakCache(HashMap<(String, String), (Instant, Option<Milestone>)>);

impl StreakCache {
    fn get(&self, key: &(String, String), now: Instant) -> Option<Option<Milestone>> {
        self.0.get(key).filter(|(at, _)| now.duration_since(*at) < STREAK_AGE).map(|(_, m)| m.clone())
    }

    fn put(&mut self, key: (String, String), milestone: Option<Milestone>, now: Instant) {
        self.0.retain(|_, (at, _)| now.duration_since(*at) < STREAK_AGE);
        if self.0.len() >= STREAK_KEPT {
            self.0.clear();
        }
        self.0.insert(key, (now, milestone));
    }

    fn forget(&mut self, account: &str) {
        self.0.retain(|(a, _), _| a != account);
    }
}

static STREAKS: LazyLock<Mutex<StreakCache>> = LazyLock::new(Mutex::default);

fn streaks() -> std::sync::MutexGuard<'static, StreakCache> {
    STREAKS.lock().unwrap_or_else(|p| p.into_inner())
}

pub async fn streak(app: &AppHandle, account: &str, login: &str) -> Result<Streak, String> {
    let key = (account.to_owned(), login.to_ascii_lowercase());
    let live = crate::tabs::live::is_live(app, login);
    if let Some(milestone) = streaks().get(&key, Instant::now()) {
        return Ok(Streak { milestone, live });
    }
    let c = call(app, account)?;
    let milestone = milestones::shareable_streak(&c.http, c.auth(), login).await.map_err(failed)?;
    streaks().put(key, milestone.clone(), Instant::now());
    Ok(Streak { milestone, live })
}

/// V108-N23 Only from the user's click on Share.
pub async fn share_streak(app: &AppHandle, account: &str, channel_id: &str, milestone_id: &str) -> Result<(), String> {
    let c = call(app, account)?;
    let out = milestones::share(&c.http, c.auth(), channel_id, milestone_id)
        .await
        .map_err(|e| match e {
            cyex_twitch::Error::Refused(code) => milestones::explain(&code).to_owned(),
            e => failed(e),
        });
    // The shared milestone is gone; the next read asks Twitch.
    streaks().forget(account);
    out
}

/// V108-N23 Follows (or stops following) the account's milestones topic while a chat shows
/// the prompt. The topic is counted, so several chats can share it; held per window
/// (`label`), so a page that goes away without its cleanup lets go too.
pub fn watch_streak(app: &AppHandle, label: &str, account: &str, on: bool) {
    crate::hermes::hold(app, label, crate::hermes::Held::User(account.to_owned(), Kind::ViewerMilestones), on);
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
    // The read again must reach Twitch (the cache is by login, the topic says the id).
    streaks().forget(account);
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

    #[test]
    fn streak_reads_kept_a_minute_and_bounded() {
        let mut c = StreakCache::default();
        let t = Instant::now();
        let key = |a: &str, l: &str| (a.to_owned(), l.to_owned());
        c.put(key("1", "x"), None, t);
        assert_eq!(c.get(&key("1", "x"), t + Duration::from_secs(59)), Some(None));
        assert_eq!(c.get(&key("1", "x"), t + STREAK_AGE), None);
        // Expired reads go on the next put.
        c.put(key("1", "y"), None, t + STREAK_AGE);
        assert_eq!(c.0.len(), 1);
        for i in 0..STREAK_KEPT * 2 {
            c.put(key("2", &i.to_string()), None, t + STREAK_AGE);
        }
        assert!(c.0.len() <= STREAK_KEPT);
        c.forget("2");
        assert!(c.0.keys().all(|(a, _)| a != "2"));
    }
}
