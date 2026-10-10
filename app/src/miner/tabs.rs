//! The app's own tabs and the miner. W1: a live stream playing in the app with a GQL
//! account sends `minute-watched` once a minute, like twitch.tv's player; the miner then
//! uses only the slots left. C22: every tab account follows its own points topic, so
//! chests in its open chats get claimed, and each new chat is checked for one waiting.
//! Also the chats the miner sits in (chat presence).

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use cyex_settings::Partition;
use cyex_settings::schema::{ChatPresence, Tabs};
use cyex_twitch::gql::TV_USER_AGENT;
use cyex_twitch::gql::miner as gm;
use cyex_twitch::hermes::Kind;
use cyex_twitch::spade;
use tauri::{AppHandle, Manager};

use super::{gql_auth, http, lock};
use crate::settings::Settings;

const MINUTE: Duration = Duration::from_secs(60);
/// New chats checked for a waiting chest per minute.
const CHECKS: usize = 10;

#[derive(Default)]
struct TabsState {
    /// Accounts whose points topic is followed for C22.
    accounts: Mutex<HashSet<String>>,
    /// (account, channel login) pairs already checked for a chest.
    checked: Mutex<HashSet<(String, String)>>,
    /// The chats the miner sat in last time.
    presence: Mutex<Vec<(Option<String>, String)>>,
}

pub fn spawn(app: &AppHandle) {
    app.manage(TabsState::default());
    on_changed(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(MINUTE).await;
            watching(&app).await;
            check_chests(&app).await;
        }
    });
}

/// Tabs or settings changed: follow the points topics of the accounts in tabs now.
pub fn on_changed(app: &AppHandle) {
    let Some(state) = app.try_state::<TabsState>() else {
        return;
    };
    let s = super::settings(app);
    let want: HashSet<String> = if s.claim_in_tabs {
        tab_pairs(app)
            .into_iter()
            .map(|(a, _)| a)
            .filter(|a| crate::auth::gql_token(app, a).is_some())
            .collect()
    } else {
        HashSet::new()
    };
    let mut have = state.accounts.lock().unwrap_or_else(|p| p.into_inner());
    for a in want.difference(&have) {
        crate::hermes::add_user(app, a, Kind::UserPoints);
    }
    for a in have.difference(&want) {
        crate::hermes::remove_user(app, a, Kind::UserPoints);
    }
    *have = want;
}

/// (account, channel login) of every tile with an account.
fn tab_pairs(app: &AppHandle) -> Vec<(String, String)> {
    let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
    tabs.tiles()
        .filter(|t| !t.channel.login.is_empty())
        .filter_map(|t| Some((t.account.clone()?, t.channel.login.clone())))
        .collect()
}

/// C22 Whether a channel is open in a tab with this account.
pub fn open_with(app: &AppHandle, account: &str, channel_id: &str) -> bool {
    let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
    tabs.tiles()
        .any(|t| t.channel.id == channel_id && t.account.as_deref() == Some(account))
}

/// W1 One minute for every live stream playing in the app with a GQL account.
async fn watching(app: &AppHandle) {
    let s = super::settings(app);
    let playing: Vec<(String, String)> = if s.watching_earns {
        crate::player::playing_live(app)
            .into_iter()
            .filter_map(|(login, account)| Some((login, account?)))
            .filter(|(_, a)| crate::auth::gql_token(app, a).is_some())
            .collect()
    } else {
        Vec::new()
    };
    let mut mine = Vec::new();
    if !playing.is_empty() {
        let logins: Vec<String> = playing.iter().map(|p| p.0.clone()).collect();
        let found = gm::streams_by_logins(&http(app), &logins)
            .await
            .unwrap_or_default();
        let url = super::spade_url(app).await;
        let miner = lock(app).account.clone();
        for (login, account) in &playing {
            let Some((c, stream)) = found
                .iter()
                .find(|c| c.login == *login)
                .and_then(|c| Some((c, c.stream.as_ref()?)))
            else {
                continue;
            };
            let minute = spade::Minute {
                channel_id: &c.id,
                login,
                broadcast_id: &stream.id,
                user_id: account,
                game: stream.game.as_ref().map(|(i, n)| (i.as_str(), n.as_str())),
            };
            match spade::send(&http(app), &url, &spade::payload(&minute), TV_USER_AGENT).await {
                Ok(204) => {}
                Ok(status) => eprintln!("miner: watch time for {login}: {status}"),
                Err(e) => eprintln!("miner: watch time for {login}: {e}"),
            }
            if miner.as_deref() == Some(account.as_str()) {
                mine.push(c.id.clone());
            }
        }
    }
    let changed = {
        let mut st = lock(app);
        mine.sort();
        let changed = st.watching != mine;
        st.watching = mine;
        changed
    };
    if changed {
        super::watch::topics(app);
        super::emit_status(app);
    }
}

/// C22 Chats new to the tabs: a chest may be waiting already.
async fn check_chests(app: &AppHandle) {
    if !super::settings(app).claim_in_tabs {
        return;
    }
    let Some(state) = app.try_state::<TabsState>() else {
        return;
    };
    let new: Vec<(String, String)> = {
        let mut checked = state.checked.lock().unwrap_or_else(|p| p.into_inner());
        let new: Vec<(String, String)> = tab_pairs(app)
            .into_iter()
            .filter(|p| !checked.contains(p))
            .take(CHECKS)
            .collect();
        checked.extend(new.iter().cloned());
        new
    };
    for (account, login) in new {
        let Some((token, client_id)) = crate::auth::gql_token(app, &account) else {
            continue;
        };
        if let Ok(Some(ctx)) = gm::points_context(&http(app), gql_auth(&token, &client_id), &login).await
            && let Some(claim) = &ctx.claim
        {
            super::claims::bonus(app, &account, &ctx.channel_id, claim).await;
        }
    }
}

/// The chats the miner sits in: its live channels, or all of them (as the mining
/// account). Part of what the tabs keep joined.
pub fn chat_rooms(app: &AppHandle) -> Vec<(Option<String>, String)> {
    let Some(miner) = app.try_state::<super::Miner>() else {
        return Vec::new();
    };
    let presence = super::settings(app).chat;
    if presence == ChatPresence::Never {
        return Vec::new();
    }
    let st = miner.state.lock().unwrap_or_else(|p| p.into_inner());
    let Some(account) = st.account.clone() else {
        return Vec::new();
    };
    st.channels
        .iter()
        .filter(|c| presence == ChatPresence::Always || c.stream.is_some())
        .map(|c| (Some(account.clone()), c.login.clone()))
        .collect()
}

/// Joins or leaves the miner's chats when they changed.
pub fn sync_presence(app: &AppHandle) {
    let Some(state) = app.try_state::<TabsState>() else {
        return;
    };
    let now = chat_rooms(app);
    let changed = {
        let mut last = state.presence.lock().unwrap_or_else(|p| p.into_inner());
        let changed = *last != now;
        *last = now;
        changed
    };
    if changed {
        crate::tabs::sync_rooms(app);
    }
}
