//! 1.57, V102-21 Our role in every open channel: the one source every mod-only control
//! reads, here and in the UI. Rooms keep it from USERSTATE and our own lines; this adds
//! what IRC never says (being modded or unmodded while a chat is open): Twitch's public
//! moderated-channel answer every 30 s, and mod/unmod events and revoked mod
//! subscriptions from the mod feeds. Every change goes to all windows as `mod:roles`.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use cyex_chat::model::ModRole;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;
use ts_rs::TS;

use super::{Chat, Identity, queue};

const CHECK_EVERY: Duration = Duration::from_secs(30);
/// Rooms that join together are checked together.
const SETTLE: Duration = Duration::from_secs(2);

/// One account's role in one channel, for the UI.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleEntry {
    pub account: String,
    pub channel: String,
    pub role: ModRole,
}

#[derive(Default)]
struct Roles {
    /// What the windows were told last.
    sent: Mutex<Vec<RoleEntry>>,
    /// The check's last answer per (account, channel). Only a change acts, so a stale
    /// answer never overrules a fresh USERSTATE.
    answers: Mutex<HashMap<(String, String), bool>>,
    wake: Notify,
}

pub fn init(app: &AppHandle) {
    app.manage(Roles::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let woken = {
                let roles = app.state::<Roles>();
                tokio::select! {
                    () = tokio::time::sleep(CHECK_EVERY) => false,
                    () = roles.wake.notified() => true,
                }
            };
            if woken {
                tokio::time::sleep(SETTLE).await;
            }
            check(&app).await;
        }
    });
}

/// The account's role in a channel, `None` when it has no mod rights there or no open room.
pub fn role(app: &AppHandle, account: &str, login: &str) -> Option<ModRole> {
    let chat = app.try_state::<Chat>()?;
    let key = (Identity::Account(account.to_owned()), login.to_ascii_lowercase());
    chat.lock().rooms.get(&key)?.room.role()
}

/// Every open account's role, by channel.
pub fn snapshot(app: &AppHandle) -> Vec<RoleEntry> {
    let Some(chat) = app.try_state::<Chat>() else {
        return Vec::new();
    };
    let mut list: Vec<RoleEntry> = chat
        .lock()
        .rooms
        .iter()
        .filter_map(|((identity, login), e)| {
            let Identity::Account(account) = identity else {
                return None;
            };
            Some(RoleEntry {
                account: account.clone(),
                channel: login.clone(),
                role: e.room.role()?,
            })
        })
        .collect();
    list.sort();
    list
}

/// Tells every window the roles, when they changed since last time. Cheap to call after
/// anything that may have changed them (joins, leaves, USERSTATE).
pub fn emit(app: &AppHandle) {
    let Some(roles) = app.try_state::<Roles>() else {
        return;
    };
    let now = snapshot(app);
    {
        let mut sent = roles.sent.lock().unwrap_or_else(|p| p.into_inner());
        if *sent == now {
            return;
        }
        sent.clone_from(&now);
    }
    let _ = app.emit("mod:roles", now);
}

/// After a role changed in one room: the join budget (mods don't count), the counter,
/// the mod feeds (1.46) and the windows.
pub fn changed(app: &AppHandle, account: &str, login: &str, privileged: bool) {
    super::budget(app, |b| {
        if let Some(b) = b.get_mut(account) {
            b.set_privileged(login, privileged);
        }
    });
    crate::connections::notify(app);
    crate::moderation::feeds::sync(app);
    emit(app);
}

/// Asks Twitch about new rooms soon instead of at the next round.
pub fn check_soon(app: &AppHandle) {
    if let Some(roles) = app.try_state::<Roles>() {
        roles.wake.notify_one();
    }
}

/// Twitch says the account moderates a channel now, or not anymore.
pub fn set_moderator(app: &AppHandle, account: &str, login: &str, on: bool) {
    let Some(chat) = app.try_state::<Chat>() else {
        return;
    };
    let key = (Identity::Account(account.to_owned()), login.to_ascii_lowercase());
    let change = {
        let mut inner = chat.lock();
        let Some(entry) = inner.rooms.get_mut(&key) else {
            return;
        };
        let before = entry.room.role();
        let ev = entry.room.set_moderator(on);
        let after = entry.room.role();
        queue(entry, ev, &chat.wake);
        (before != after).then_some(after.is_some())
    };
    if let Some(privileged) = change {
        changed(app, account, &key.1, privileged);
    }
}

/// Like `set_moderator`, for news that names the channel by id (EventSub).
pub fn set_moderator_in(app: &AppHandle, account: &str, channel_id: &str, on: bool) {
    let Some(chat) = app.try_state::<Chat>() else {
        return;
    };
    let login = chat.lock().rooms.iter().find_map(|((identity, login), e)| {
        (*identity == Identity::Account(account.to_owned())
            && e.room.state.channel_id.as_deref() == Some(channel_id))
        .then(|| login.clone())
    });
    if let Some(login) = login {
        set_moderator(app, account, &login, on);
    }
}

/// One round of Twitch's moderated-channel answer for every room an account reads.
async fn check(app: &AppHandle) {
    // Account → (login, channel id, Twitch has told us about ourselves yet).
    let mut asks: HashMap<String, Vec<(String, String, bool)>> = HashMap::new();
    {
        let Some(chat) = app.try_state::<Chat>() else {
            return;
        };
        for ((identity, login), e) in &chat.lock().rooms {
            let Identity::Account(account) = identity else {
                continue;
            };
            let Some(cid) = e.room.state.channel_id.clone() else {
                continue;
            };
            if e.room.owner() {
                continue;
            }
            asks.entry(account.clone())
                .or_default()
                .push((login.clone(), cid, e.room.state.own.is_some()));
        }
    }
    let http = app.state::<crate::auth::Auth>().http.clone();
    let mut seen = Vec::new();
    for (account, rooms) in asks {
        for chunk in rooms.chunks(cyex_twitch::gql::viewer::MODERATES_MAX) {
            let ids: Vec<String> = chunk.iter().map(|r| r.1.clone()).collect();
            let Ok(answers) = cyex_twitch::gql::viewer::moderates(&http, &account, &ids).await else {
                continue;
            };
            for ((login, _, known), on) in chunk.iter().zip(answers) {
                let key = (account.clone(), login.clone());
                seen.push(key.clone());
                let before = {
                    let roles = app.state::<Roles>();
                    let mut answers = roles.answers.lock().unwrap_or_else(|p| p.into_inner());
                    answers.insert(key, on)
                };
                if acts(before, on, *known) {
                    set_moderator(app, &account, login, on);
                }
            }
        }
    }
    let roles = app.state::<Roles>();
    roles
        .answers
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .retain(|k, _| seen.contains(k));
}

/// Whether an answer from the check changes the room: when it differs from the last one
/// (modded or unmodded since), or, the first time, when Twitch hasn't said anything else
/// yet and the answer is "mod".
fn acts(before: Option<bool>, now: bool, known: bool) -> bool {
    match before {
        Some(b) => b != now,
        None => now && !known,
    }
}

#[cfg(test)]
mod tests {
    use super::acts;

    #[test]
    fn only_changes_in_the_answer_act() {
        // First answer: only fills in a "mod" before USERSTATE came.
        assert!(acts(None, true, false));
        assert!(!acts(None, true, true));
        assert!(!acts(None, false, false));
        assert!(
            !acts(None, false, true),
            "a stale \"no\" doesn't overrule USERSTATE"
        );
        // Later: a flip is a mod or unmod while the chat is open.
        assert!(acts(Some(false), true, true));
        assert!(acts(Some(true), false, true));
        assert!(!acts(Some(true), true, true));
        assert!(!acts(Some(false), false, true));
    }
}
