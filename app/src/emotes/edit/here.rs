//! I106-3 This chat's channel as an edit target, for the chat commands.

use cyex_chat::model::Provider;
use cyex_emotes::seventv::account::sets::EmoteBrief;
use cyex_emotes::{Emote, Scope};
use tauri::{AppHandle, Manager};

use super::reach::Target;
use super::{lock, state};

/// The cached target whose `owner_id` is `channels[login].seventv_user` (Emotes state), cloned.
/// No network.
pub fn here_target(app: &AppHandle, account: &str, login: &str) -> Option<Target> {
    let (owner, _) = super::ask::viewed(app, login, "", "");
    let owner = owner?;
    let reach = lock(&state(app).reach);
    reach
        .get(account)?
        .1
        .as_ref()?
        .targets
        .iter()
        .find(|t| t.owner_id == owner)
        .cloned()
}

/// Tab for `/remove` and `/rename`: that target's aliases, sorted. Empty when not editable or
/// not cached.
pub fn here_names(app: &AppHandle, account: &str, login: &str) -> Vec<String> {
    let mut names: Vec<String> = here_target(app, account, login)
        .map(|t| t.entries.into_iter().map(|(_, alias)| alias).collect())
        .unwrap_or_default();
    names.sort_unstable();
    names.dedup();
    names
}

/// Own 7TV name, private flag, owner: a loaded emote, else 7TV. `Ok(None)` = no such emote;
/// `Err(())` = 7TV down.
#[allow(clippy::result_unit_err)]
pub async fn emote_facts(app: &AppHandle, login: &str, id: &str) -> Result<Option<EmoteBrief>, ()> {
    if let Some(e) = super::ask::find_emote(app, login, id) {
        return Ok(Some(EmoteBrief {
            id: e.id.clone(),
            name: e.original.unwrap_or(e.name),
            private: e.private,
            owner_id: e.owner_id,
        }));
    }
    crate::cosmetics::api(app).emote_brief(id).await.map_err(|e| {
        eprintln!("7tv edit: emote {id}: {e}");
    })
}

/// V111-03 A cached target by its channel's 7TV user, or by its name (a channel not open here:
/// the name 7TV gives is the Twitch display name, which is the login up to case). No network.
fn cached_target(app: &AppHandle, account: &str, owner: Option<&str>, login: &str) -> Option<Target> {
    let reach = lock(&state(app).reach);
    let targets = &reach.get(account)?.1.as_ref()?.targets;
    let by_owner = owner.and_then(|o| targets.iter().find(|t| t.owner_id == o));
    by_owner
        .or_else(|| targets.iter().find(|t| t.channel.eq_ignore_ascii_case(login)))
        .cloned()
}

/// V111-03 `#channel` as an edit target: an open chat's channel as `here_target`, else the
/// cached targets by name, else its 7TV user asked for. `None`: the account can't edit it.
pub async fn target_for(app: &AppHandle, account: &str, login: &str) -> Option<Target> {
    let (owner, _) = super::ask::viewed(app, login, "", "");
    if let Some(owner) = owner {
        return cached_target(app, account, Some(&owner), "");
    }
    if let Some(t) = cached_target(app, account, None, login) {
        return Some(t);
    }
    let id = twitch_id(app, login).await.ok()??;
    let owner = crate::emotes::seventv_user(app, &id).await?;
    cached_target(app, account, Some(&owner), "")
}

/// V111-03 Tab for `/remove` and `/rename` with `#channel`: as `here_names`, also for a channel
/// whose chat isn't open but whose emotes the account edits.
pub fn names_for(app: &AppHandle, account: &str, login: &str) -> Vec<String> {
    let (owner, _) = super::ask::viewed(app, login, "", "");
    let mut names: Vec<String> = cached_target(app, account, owner.as_deref(), login)
        .map(|t| t.entries.into_iter().map(|(_, alias)| alias).collect())
        .unwrap_or_default();
    names.sort_unstable();
    names.dedup();
    names
}

/// V111-03 Tab after `#`: the channels of open chats, and those the account edits on 7TV (by
/// a name that is a login), lower case, sorted, without `here`.
pub fn channel_names(app: &AppHandle, account: Option<&str>, here: &str) -> Vec<String> {
    let mut out: Vec<String> = {
        let st = app.state::<crate::emotes::Emotes>();
        let inner = st.lock();
        inner.channels.keys().cloned().collect()
    };
    if let Some(account) = account {
        let reach = lock(&state(app).reach);
        if let Some((_, Some(r))) = reach.get(account) {
            out.extend(r.targets.iter().map(|t| t.channel.to_ascii_lowercase()));
        }
    }
    out.retain(|l| is_login(l) && !l.eq_ignore_ascii_case(here));
    out.sort_unstable();
    out.dedup();
    out
}

/// V111-03 A Twitch login: 1-25 of a-z, 0-9 and `_`.
pub fn is_login(s: &str) -> bool {
    (1..=25).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// V111-03 A channel's Twitch id: an open chat's, else Twitch. `Ok(None)`: no such channel.
async fn twitch_id(app: &AppHandle, login: &str) -> Result<Option<String>, ()> {
    if let Some(id) = crate::emotes::channel_id(app, login) {
        return Ok(Some(id));
    }
    let http = app.state::<crate::auth::Auth>().http.clone();
    let found = cyex_twitch::gql::ids_by_login(&http, &[login.to_owned()])
        .await
        .map_err(|e| eprintln!("7tv edit: channel {login}: {e}"))?;
    Ok(found
        .into_iter()
        .find(|(l, _)| l.eq_ignore_ascii_case(login))
        .map(|(_, id)| id))
}

/// V111-03 Why a channel's 7TV emotes can't be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFail {
    /// No Twitch channel with that name.
    NoChannel,
    /// It has no 7TV emotes.
    NoSet,
    Down,
}

/// V111-03 The 7TV emotes of a channel's active set, names as there: an open chat's set, else
/// 7TV's (read without a sign-in).
pub async fn channel_emotes(app: &AppHandle, login: &str) -> Result<Vec<Emote>, SourceFail> {
    if let Some(found) = loaded_set(app, login) {
        return Ok(found);
    }
    let id = twitch_id(app, login)
        .await
        .map_err(|()| SourceFail::Down)?
        .ok_or(SourceFail::NoChannel)?;
    let http = app.state::<crate::auth::Auth>().http.clone();
    let ch = cyex_emotes::seventv::api::channel(&http, &id, cyex_twitch::pace::Lane::Front)
        .await
        .map_err(|e| {
            eprintln!("7tv edit: emotes of {login}: {e}");
            SourceFail::Down
        })?;
    let set = ch.and_then(|c| c.set).ok_or(SourceFail::NoSet)?;
    Ok(set.emotes.iter().map(|e| (**e).clone()).collect())
}

/// V111-03 An open chat's 7TV channel set, cloned. `None` also while 7TV emotes are off there.
fn loaded_set(app: &AppHandle, login: &str) -> Option<Vec<Emote>> {
    let st = app.state::<crate::emotes::Emotes>();
    let inner = st.lock();
    let set = inner
        .channels
        .get(login)?
        .sets
        .iter()
        .find(|s| s.provider == Provider::Seventv && s.scope == Scope::Channel)?
        .clone();
    Some(set.emotes.iter().map(|e| (**e).clone()).collect())
}

/// V111-03 Tab for `/steal` and `/whoadded`: an open chat's 7TV emote names, sorted. No network.
pub fn loaded_names(app: &AppHandle, login: &str) -> Vec<String> {
    let mut names: Vec<String> = loaded_set(app, login)
        .map(|s| s.into_iter().map(|e| e.name).collect())
        .unwrap_or_default();
    names.sort_unstable();
    names.dedup();
    names
}

/// V111-03 Display names of 7TV users by id. Empty when 7TV didn't answer.
pub async fn user_names(app: &AppHandle, ids: &[String]) -> std::collections::HashMap<String, String> {
    let http = app.state::<crate::auth::Auth>().http.clone();
    cyex_emotes::seventv::api::user_names(&http, ids, cyex_twitch::pace::Lane::Front)
        .await
        .unwrap_or_default()
}
