//! 2.6, F2, F6, C2 The user's rules, built from settings and shared by every room.
//! Twitch blocks (F2) come from each signed-in account when chat starts and after
//! account changes; `/block` and `/unblock` update them right away.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use cyex_chat::rules::Rules;
use cyex_chat::similar::RepeatCheck;
use cyex_settings::Partition;
use cyex_settings::schema::{Chat as ChatSettings, Highlights};
use cyex_twitch::{gql, helix};
use tauri::{AppHandle, Manager};

use super::Chat;
use crate::settings::Settings;

#[derive(Default)]
pub struct RuleState {
    rules: Mutex<Arc<Rules>>,
    /// Blocked user ids per account.
    blocked: Mutex<Vec<(String, HashSet<String>)>>,
}

pub fn init(app: &AppHandle) {
    app.manage(RuleState::default());
    rebuild(app);
    load_blocked(app.clone());
}

/// The rules rooms use right now.
pub fn current(app: &AppHandle) -> Arc<Rules> {
    let state = app.state::<RuleState>();
    let rules = state.rules.lock().unwrap_or_else(|p| p.into_inner());
    rules.clone()
}

/// After settings or the block lists changed: new rules for every room.
pub fn rebuild(app: &AppHandle) {
    let settings = app.state::<Settings>();
    let h: Highlights = settings.read(Partition::Highlights);
    let c: ChatSettings = settings.read(Partition::Chat);
    let mut rules = Rules::new(&h);
    rules.repeat = c.repeats.then(|| RepeatCheck {
        threshold: f32::from(c.repeat_similarity) / 100.0,
        min: 2,
    });
    let state = app.state::<RuleState>();
    let blocked: HashSet<String> = state
        .blocked
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .flat_map(|(_, ids)| ids.iter().cloned())
        .collect();
    rules.set_blocked(blocked);
    let rules = Arc::new(rules);
    *state.rules.lock().unwrap_or_else(|p| p.into_inner()) = rules.clone();
    if let Some(chat) = app.try_state::<Chat>() {
        for entry in chat.lock().rooms.values_mut() {
            entry.room.set_rules(rules.clone());
        }
    }
}

/// F2 Every signed-in account's block list, Helix first, else GQL.
pub fn load_blocked(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let http = app.state::<crate::auth::Auth>().http.clone();
        let mut lists = Vec::new();
        for id in crate::auth::accounts(&app).list.iter().map(|a| a.user_id.clone()) {
            let ids = if let Some((token, client_id)) =
                crate::auth::helix_scoped(&app, &id, "user:read:blocked_users")
            {
                let auth = helix::Auth {
                    client_id: &client_id,
                    token: &token,
                };
                helix::blocks::blocked_ids(&http, auth, &id).await
            } else if let Some((token, client_id)) = crate::auth::gql_token(&app, &id) {
                let auth = gql::Auth {
                    client_id: &client_id,
                    token: Some(&token),
                };
                gql::blocks::blocked_ids(&http, auth).await
            } else {
                continue;
            };
            match ids {
                Ok(ids) => lists.push((id, ids.into_iter().collect())),
                Err(e) => eprintln!("chat: no block list for {id}: {e}"),
            }
        }
        *app.state::<RuleState>()
            .blocked
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = lists;
        rebuild(&app);
    });
}

/// `/block` or `/unblock` went through.
pub fn note_block(app: &AppHandle, account: &str, target: &str, blocked: bool) {
    {
        let state = app.state::<RuleState>();
        let mut lists = state.blocked.lock().unwrap_or_else(|p| p.into_inner());
        let pos = lists.iter().position(|(a, _)| a == account).unwrap_or_else(|| {
            lists.push((account.to_owned(), HashSet::new()));
            lists.len() - 1
        });
        if blocked {
            lists[pos].1.insert(target.to_owned());
        } else {
            lists[pos].1.remove(target);
        }
    }
    rebuild(app);
}

/// Some signed-in account blocks this user.
pub fn is_blocked(app: &AppHandle, user_id: &str) -> bool {
    let state = app.state::<RuleState>();
    let lists = state.blocked.lock().unwrap_or_else(|p| p.into_inner());
    lists.iter().any(|(_, ids)| ids.contains(user_id))
}
