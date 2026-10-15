//! C4 Polls and predictions as a viewer, and on-screen celebrations (1.42), for the chats
//! on screen. Read once from GQL when a chat opens, then kept current over Hermes
//! (anonymous, `crate::hermes`). Your own votes and bets come from the chat's GQL sign-in.

use std::collections::HashMap;
use std::sync::Mutex;

use cyex_twitch::gql::{self, polls::Poll, predictions::Prediction};
use cyex_twitch::hermes::{Event, Kind};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::run::Place;

const EVENT: &str = "chat:votes";
const KINDS: [Kind; 3] = [Kind::Polls, Kind::Predictions, Kind::Celebrations];
const CELEBRATION: &str = "player:celebration";

#[derive(Clone, Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VotesUpdate {
    /// Channel login.
    pub channel: String,
    pub poll: Option<Poll>,
    pub prediction: Option<Prediction>,
    /// Points to bet with, when the chat's account is signed in.
    #[ts(type = "number | null")]
    pub balance: Option<u64>,
    /// Twitch's prediction terms are accepted.
    pub accepted: bool,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Celebration {
    pub channel: String,
    pub emote_id: String,
}

struct Watched {
    id: String,
    views: usize,
    state: VotesUpdate,
}

pub struct Votes {
    /// By channel login.
    channels: Mutex<HashMap<String, Watched>>,
}

pub fn init(app: &AppHandle) {
    app.manage(Votes {
        channels: Mutex::default(),
    });
}

fn gql_auth(app: &AppHandle, account: Option<&str>) -> Option<(String, String)> {
    account.and_then(|a| crate::auth::gql_token(app, a))
}

/// A chat view opened.
pub fn watch(app: &AppHandle, login: &str, account: Option<String>) {
    {
        let votes = app.state::<Votes>();
        let mut map = votes.channels.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(w) = map.get_mut(login) {
            w.views += 1;
            return;
        }
        map.insert(
            login.to_owned(),
            Watched {
                id: String::new(),
                views: 1,
                state: VotesUpdate {
                    channel: login.to_owned(),
                    ..VotesUpdate::default()
                },
            },
        );
    }
    let (app, login) = (app.clone(), login.to_owned());
    tauri::async_runtime::spawn(async move { load(&app, &login, account.as_deref(), true).await });
}

/// A chat view closed.
pub fn unwatch(app: &AppHandle, login: &str) {
    let votes = app.state::<Votes>();
    let mut map = votes.channels.lock().unwrap_or_else(|p| p.into_inner());
    let Some(w) = map.get_mut(login) else { return };
    w.views = w.views.saturating_sub(1);
    if w.views == 0
        && let Some(w) = map.remove(login)
        && !w.id.is_empty()
    {
        for kind in KINDS {
            crate::hermes::remove(app, kind, &w.id);
        }
    }
}

/// Reads the poll and prediction; subscribes the first time.
async fn load(app: &AppHandle, login: &str, account: Option<&str>, subscribe: bool) {
    let http = app.state::<crate::auth::Auth>().http.clone();
    let token = gql_auth(app, account);
    let auth = token.as_ref().map_or(gql::Auth::PUBLIC, |(t, c)| gql::Auth {
        client_id: c,
        token: Some(t),
    });
    let (id, poll) = match gql::polls::poll(&http, auth, login).await {
        Ok(Some(found)) => found,
        _ => return,
    };
    let pred = gql::predictions::prediction(&http, auth, login)
        .await
        .unwrap_or_default();
    let update = {
        let votes = app.state::<Votes>();
        let mut map = votes.channels.lock().unwrap_or_else(|p| p.into_inner());
        let Some(w) = map.get_mut(login) else { return };
        if subscribe && w.id.is_empty() {
            for kind in KINDS {
                crate::hermes::add(app, kind, &id);
            }
        }
        w.id = id;
        w.state.poll = poll.filter(|p| p.status != "ARCHIVED");
        w.state.prediction = pred.prediction;
        w.state.balance = pred.balance;
        w.state.accepted = pred.accepted;
        w.state.clone()
    };
    let _ = app.emit(EVENT, update);
}

/// What a chat shows now.
pub fn state(app: &AppHandle, login: &str) -> Option<VotesUpdate> {
    let votes = app.state::<Votes>();
    let map = votes.channels.lock().unwrap_or_else(|p| p.into_inner());
    map.get(login).map(|w| w.state.clone())
}

/// A Hermes event for polls, predictions or celebrations (`crate::hermes`).
pub fn on_event(app: &AppHandle, e: &Event) {
    let votes = app.state::<Votes>();
    let mut map = votes.channels.lock().unwrap_or_else(|p| p.into_inner());
    let Some((login, w)) = map.iter_mut().find(|(_, w)| w.id == e.topic.channel_id) else {
        return;
    };
    match e.topic.kind {
        Kind::Celebrations => {
            if let Some(emote) = e.payload.pointer("/data/emote_id").and_then(|v| v.as_str()) {
                let _ = app.emit(
                    CELEBRATION,
                    Celebration {
                        channel: login.clone(),
                        emote_id: emote.to_owned(),
                    },
                );
            }
            return;
        }
        Kind::Polls => {
            let mine = w.state.poll.as_ref().map(|p| p.mine.clone()).unwrap_or_default();
            let poll = Poll::from_hermes(&e.payload, mine);
            w.state.poll = poll.filter(|p| p.status != "ARCHIVED");
        }
        Kind::Predictions => {
            let mine = w.state.prediction.as_ref().and_then(|p| p.mine.clone());
            let Some(p) = Prediction::from_hermes(&e.payload, mine) else {
                return;
            };
            // Updates come out of order: keep the newest.
            let stale = w
                .state
                .prediction
                .as_ref()
                .is_some_and(|old| old.id == p.id && old.updated > p.updated);
            if stale {
                return;
            }
            w.state.prediction = Some(p);
        }
        _ => return,
    }
    let _ = app.emit(EVENT, w.state.clone());
}

fn view_key(app: &AppHandle, view: u32) -> Result<super::RoomKey, String> {
    let chat = app.state::<super::Chat>();
    let inner = chat.lock();
    inner
        .views
        .get(&view)
        .map(|v| v.0.clone())
        .ok_or_else(|| "This chat is closed.".into())
}

fn signed_in(app: &AppHandle, key: &super::RoomKey) -> Result<(String, (String, String)), String> {
    let super::Identity::Account(account) = &key.0 else {
        return Err("Sign in to vote.".into());
    };
    let token =
        crate::auth::gql_token(app, account).ok_or("Voting needs the GQL sign-in (Settings → Accounts).")?;
    Ok((account.clone(), token))
}

/// One vote, or with `points` an extra one bought with channel points.
pub async fn vote(app: &AppHandle, view: u32, choice: &str, points: Option<u32>) -> Result<(), String> {
    let key = view_key(app, view)?;
    let (account, (token, client_id)) = signed_in(app, &key)?;
    let poll_id = state(app, &key.1)
        .and_then(|s| s.poll)
        .map(|p| p.id)
        .ok_or("There’s no poll right now.")?;
    let http = app.state::<crate::auth::Auth>().http.clone();
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    if let Some(code) = gql::polls::vote(&http, auth, &poll_id, choice, &account, points)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err(vote_error(&code));
    }
    load(app, &key.1, Some(&account), false).await;
    Ok(())
}

/// `/vote 2`.
pub async fn vote_index(app: &AppHandle, place: &Place, n: usize) -> Result<(), String> {
    let view = place.view.ok_or("Open the chat to vote.")?;
    let choice = state(app, place.channel())
        .and_then(|s| s.poll)
        .and_then(|p| p.choices.get(n.wrapping_sub(1)).map(|c| c.id.clone()))
        .ok_or("No such choice.")?;
    vote(app, view, &choice, None).await
}

pub async fn predict(app: &AppHandle, view: u32, outcome: &str, points: u64) -> Result<(), String> {
    let key = view_key(app, view)?;
    let (account, (token, client_id)) = signed_in(app, &key)?;
    let s = state(app, &key.1).ok_or("This chat is closed.")?;
    let event = s
        .prediction
        .map(|p| p.id)
        .ok_or("There’s no prediction right now.")?;
    let http = app.state::<crate::auth::Auth>().http.clone();
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    if !s.accepted {
        gql::predictions::accept_terms(&http, auth)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(code) = gql::predictions::predict(&http, auth, &event, outcome, points)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err(predict_error(&code));
    }
    load(app, &key.1, Some(&account), false).await;
    Ok(())
}

fn vote_error(code: &str) -> String {
    match code {
        "POLL_NOT_ACTIVE" | "POLL_NOT_FOUND" => "The poll is over.".into(),
        "TOKENS_REQUIRED" => "You voted already. More votes cost channel points.".into(),
        "MULTI_CHOICE_VOTE_FORBIDDEN" => "Only one choice in this poll.".into(),
        "INSUFFICIENT_COMMUNITY_POINTS_BALANCE" => "Not enough channel points.".into(),
        "RATE_LIMITED" => "Slow down a little.".into(),
        other => format!("Twitch said no ({other})."),
    }
}

fn predict_error(code: &str) -> String {
    match code {
        "EVENT_NOT_ACTIVE" => "Predictions are closed.".into(),
        "NOT_ENOUGH_POINTS" => "Not enough channel points.".into(),
        "MAX_POINTS_PER_EVENT" => "That’s more than this prediction allows.".into(),
        "MULTIPLE_OUTCOMES" => "You already picked the other side.".into(),
        "REGION_LOCKED" | "CATEGORY_REGION_LOCKED" => "Predictions aren’t available where you are.".into(),
        other => format!("Twitch said no ({other})."),
    }
}
