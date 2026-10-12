//! Fetching sets: global ones once per run, a channel's when its chat joins, and an
//! account's own Twitch emotes (GQL sign-in first, else Helix with `user:read:emotes`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use cyex_chat::model::Provider;
use cyex_emotes::seventv::{self, socket::Topic as SevenTopic};
use cyex_emotes::{Emote, EmoteSet, Lookup, Scope};
use cyex_twitch::gql::{self, EmoteSet as TwitchSet};
use cyex_twitch::{Http, helix};
use tauri::{AppHandle, Emitter, Manager};

use super::Emotes;
use crate::auth::{Auth, gql_token, helix_scoped};

/// An account's Twitch emotes are fetched again after this.
const MINE_FRESH: Duration = Duration::from_secs(300);

fn http(app: &AppHandle) -> Http {
    app.state::<Auth>().http.clone()
}

/// Global sets, once. Lookups are rebuilt when they arrive.
pub(super) fn globals(app: &AppHandle) {
    let state = app.state::<Emotes>();
    {
        let mut inner = state.lock();
        if inner.globals_loading {
            return;
        }
        inner.globals_loading = true;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let http = http(&app);
        let mut sets = Vec::new();
        match seventv::api::global(&http).await {
            Ok(s) => sets.push(Arc::new(s)),
            Err(e) => eprintln!("emotes: no 7TV globals: {e}"),
        }
        sets.extend(super::providers::globals(&app, &http).await);
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        if sets.is_empty() {
            // Try again with the next channel; meanwhile nobody waits for them.
            inner.globals_loading = false;
            drop(inner);
            let _ = state.globals_ready.send(true);
            return;
        }
        inner.globals = sets;
        inner.rebuild_all();
        drop(inner);
        let _ = state.globals_ready.send(true);
    });
}

pub fn channel(app: AppHandle, login: String, id: String) {
    globals(&app);
    tauri::async_runtime::spawn(async move {
        let http = http(&app);
        let (seven, others) = tokio::join!(
            seventv::api::channel(&http, &id),
            super::providers::channel(&http, &id)
        );
        let state = app.state::<Emotes>();
        let mut topics = Vec::new();
        {
            let mut inner = state.lock();
            let Some(c) = inner.channels.get_mut(&login).filter(|c| c.id == id) else {
                return;
            };
            match seven {
                Ok(Some(ch)) => {
                    topics.push(SevenTopic::User(ch.user_id.clone()));
                    c.seventv_user = Some(ch.user_id);
                    if let Some(set) = ch.set {
                        topics.push(SevenTopic::Set(set.id.clone()));
                        c.sets.push(Arc::new(set));
                    }
                }
                Ok(None) => {}
                Err(e) => eprintln!("emotes: no 7TV emotes for {login}: {e}"),
            }
            c.sets.extend(others.0);
            c.ffz_badges = others.1;
            let _ = c.ready.send(true);
            inner.rebuild(&login);
        }
        for t in topics {
            state.seventv.add(t);
        }
        // Windows refresh what depends on it, e.g. FFZ's mod and VIP pictures.
        let _ = app.emit("emotes:loaded", &login);
    });
}

/// Replaces a channel's 7TV set, e.g. after the channel switched sets.
pub fn seventv_set(app: AppHandle, login: String, set_id: Option<String>) {
    tauri::async_runtime::spawn(async move {
        let set = match &set_id {
            Some(id) => match seventv::api::sets(&http(&app), std::slice::from_ref(id)).await {
                Ok(mut s) if !s.is_empty() => {
                    let mut set = s.remove(0);
                    set.scope = Scope::Channel;
                    Some(set)
                }
                _ => return,
            },
            None => None,
        };
        let state = app.state::<Emotes>();
        let old: Vec<String> = {
            let mut inner = state.lock();
            let Some(c) = inner.channels.get_mut(&login) else {
                return;
            };
            let old = c
                .sets
                .iter()
                .filter(|s| s.provider == Provider::Seventv)
                .map(|s| s.id.clone())
                .collect();
            c.sets.retain(|s| s.provider != Provider::Seventv);
            c.sets.extend(set.map(Arc::new));
            inner.rebuild(&login);
            old
        };
        for id in old {
            state.seventv.remove(&SevenTopic::Set(id));
        }
        if let Some(id) = set_id {
            state.seventv.add(SevenTopic::Set(id));
        }
    });
}

/// The Twitch sets an account can use in a channel (1.40), cached for a few minutes.
/// Also keeps the account's lookup for its own lines.
pub async fn twitch_sets(
    app: &AppHandle,
    account: &str,
    channel_id: &str,
    fresh: bool,
) -> Result<Arc<Vec<TwitchSet>>, String> {
    let state = app.state::<Emotes>();
    let key = (account.to_owned(), channel_id.to_owned());
    if !fresh
        && let Some((at, sets)) = state.lock().twitch_sets.get(&key)
        && at.elapsed() < MINE_FRESH
    {
        return Ok(sets.clone());
    }
    let http = http(app);
    let sets = if let Some((token, client_id)) = gql_token(app, account) {
        let auth = gql::Auth {
            client_id: &client_id,
            token: Some(&token),
        };
        gql::available_emotes(&http, auth, channel_id).await
    } else if let Some((token, client_id)) = helix_scoped(app, account, "user:read:emotes") {
        let auth = helix::Auth {
            client_id: &client_id,
            token: &token,
        };
        let mut sets = helix::user_emotes(&http, auth, account, channel_id).await;
        if let Ok(sets) = &mut sets {
            name_owners(&http, sets).await;
        }
        sets
    } else {
        return Err("Sign in to use your Twitch emotes.".into());
    }
    .map_err(|e| e.to_string())?;
    let sets = Arc::new(sets);
    let mut inner = state.lock();
    inner.twitch_sets.insert(key, (Instant::now(), sets.clone()));
    let mine: Vec<EmoteSet> = inner
        .twitch_sets
        .iter()
        .filter(|((a, _), _)| a == account)
        .flat_map(|(_, (_, s))| s.iter())
        .map(|s| {
            let emotes = s.emotes.iter().map(twitch_emote).collect();
            EmoteSet::new(s.id.clone(), Provider::Twitch, Scope::Global, emotes)
        })
        .collect();
    inner
        .mine
        .insert(account.to_owned(), Arc::new(Lookup::build(mine.iter())));
    Ok(sets)
}

/// Loads the account's Twitch emotes in the background, once per run unless `fresh`.
pub fn mine(app: AppHandle, account: String, channel_id: String, fresh: bool) {
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        if !fresh && (inner.mine.contains_key(&account) || !inner.mine_loading.insert(account.clone())) {
            return;
        }
    }
    tauri::async_runtime::spawn(async move {
        if let Err(e) = twitch_sets(&app, &account, &channel_id, fresh).await {
            eprintln!("emotes: no Twitch emotes for {account}: {e}");
        }
        app.state::<Emotes>().lock().mine_loading.remove(&account);
    });
}

pub fn twitch_emote(e: &gql::TwitchEmote) -> Emote {
    let mut emote = Emote::new(Provider::Twitch, e.id.clone(), e.name.clone());
    emote.animated = e.animated;
    emote
}

/// Helix only gives owner ids; names come from the public user lookup.
async fn name_owners(http: &Http, sets: &mut [TwitchSet]) {
    let mut ids: Vec<String> = sets
        .iter()
        .filter_map(|s| s.owner.as_ref().map(|o| o.id.clone()))
        .collect();
    ids.sort();
    ids.dedup();
    let Ok(cards) = gql::cards(http, &ids).await else {
        return;
    };
    for s in sets.iter_mut() {
        if let Some(o) = &mut s.owner
            && let Some(c) = cards.iter().find(|c| c.id == o.id)
        {
            o.login.clone_from(&c.login);
            o.name.clone_from(&c.name);
            o.avatar.clone_from(&c.avatar);
        }
    }
}
