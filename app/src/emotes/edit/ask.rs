//! V106-31/32 What the menus offer, without network.

use cyex_chat::model::Provider;
use cyex_emotes::{Emote, Scope};
use tauri::{AppHandle, Manager};

use super::options::{self as pure, Facts, Viewed, set_alias};
use super::warm::reachability;
use super::{EditOptions, NameCheck, lock, state};

/// What the menus offer for this emote in this chat. No network.
pub fn options(app: &AppHandle, account: &str, login: &str, emote_id: &str, name: &str) -> EditOptions {
    let now = reachability(app, account);
    let found = find_emote(app, login, emote_id);
    let own_name = found
        .as_ref()
        .map_or(name, |e| e.original.as_deref().unwrap_or(&e.name));
    let facts = Facts {
        seventv: found.as_ref().map_or(cyex_emotes::seventv::is_id(emote_id), |e| {
            e.provider == Provider::Seventv
        }),
        emote_id,
        own_name,
        private: found.as_ref().is_some_and(|e| e.private),
        owner_id: found.as_ref().and_then(|e| e.owner_id.as_deref()),
    };
    let (owner, set) = viewed(app, login, emote_id, name);
    let viewed = Viewed {
        owner_id: owner.as_deref(),
        set_id: set.as_ref().map(|(id, _)| id.as_str()),
        alias: set.as_ref().and_then(|(_, a)| a.as_deref()),
    };
    let reach = lock(&state(app).reach);
    let r = reach.get(account).and_then(|(_, r)| r.as_ref());
    pure::options(now, r, &viewed, &facts)
}

/// This chat's channel: its 7TV user, and its channel set with the emote's name there (rc-2:
/// by id, so a line from before a rename gets the current name).
pub(super) fn viewed(
    app: &AppHandle,
    login: &str,
    id: &str,
    name: &str,
) -> (Option<String>, Option<(String, Option<String>)>) {
    let st = app.state::<crate::emotes::Emotes>();
    let inner = st.lock();
    let Some(c) = inner.channels.get(login) else {
        return (None, None);
    };
    let set = c
        .sets
        .iter()
        .find(|s| s.provider == Provider::Seventv && s.scope == Scope::Channel)
        .map(|s| {
            let entries = s.emotes.iter().map(|e| (e.id.as_str(), e.name.as_str()));
            (s.id.clone(), set_alias(entries, id, name).map(str::to_owned))
        });
    (c.seventv_user.clone(), set)
}

/// The emote by id: this chat's sets, the global ones, personal sets, then other chats. A clone.
pub(super) fn find_emote(app: &AppHandle, login: &str, emote_id: &str) -> Option<Emote> {
    let st = app.state::<crate::emotes::Emotes>();
    let inner = st.lock();
    let here = inner.channels.get(login).into_iter().flat_map(|c| c.sets.iter());
    let others = inner
        .channels
        .iter()
        .filter(|(l, _)| *l != login)
        .flat_map(|(_, c)| c.sets.iter());
    here.chain(inner.globals.iter())
        .chain(inner.personal_sets.values())
        .chain(others)
        .flat_map(|s| s.emotes.iter())
        .find(|e| e.id == emote_id)
        .map(|e| (**e).clone())
}

/// Whether `alias` is free in the set. Unknown set: free (the server decides).
pub fn name_check(
    app: &AppHandle,
    account: &str,
    set_id: &str,
    alias: &str,
    rename_from: Option<&str>,
) -> NameCheck {
    if rename_from == Some(alias) {
        return NameCheck::Same;
    }
    let reach = lock(&state(app).reach);
    let taken = reach
        .get(account)
        .and_then(|(_, r)| r.as_ref())
        .and_then(|r| r.targets.iter().find(|t| t.set_id == set_id))
        .is_some_and(|t| t.taken(alias));
    if taken { NameCheck::Taken } else { NameCheck::Free }
}
