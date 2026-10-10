//! 1.26 Twitch's bell for the default account: the unseen count when the app starts and
//! as it changes (Hermes), the list when opened, read and delete. Items Twitch marks as
//! pop-ups become Windows toasts when that's on (Settings → Notifications). Needs a GQL
//! sign-in.

use std::sync::Mutex;

use cyex_settings::Partition;
use cyex_settings::schema::Notifications;
use cyex_twitch::gql::notifications::{self as api, Item, List, Live};
use cyex_twitch::hermes::{Event, Kind};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::settings::Settings;
use crate::toasts::{self, Open, Pop};
use crate::whispers::GqlCall;

const UNSEEN: &str = "bell:unseen";
const LIVE: &str = "bell:live";
const NO_ACCOUNT: &str = "The bell needs a GQL sign-in (Settings → Accounts).";

#[derive(Default)]
struct Inner {
    account: Option<String>,
    unseen: u32,
}

#[derive(Default)]
pub struct Bell(Mutex<Inner>);

/// The badge, for a window that just opened.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BellState {
    pub available: bool,
    pub unseen: u32,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BellPage {
    pub items: Vec<Item>,
    pub cursor: Option<String>,
}

pub fn init(app: &AppHandle) {
    app.manage(Bell::default());
    accounts_changed(app);
}

fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, Inner> {
    app.state::<Bell>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// Follows the default account's bell when it has a GQL sign-in.
pub fn accounts_changed(app: &AppHandle) {
    if app.try_state::<Bell>().is_none() {
        return;
    }
    let wanted = crate::auth::accounts(app)
        .default_id
        .filter(|id| crate::auth::gql_token(app, id).is_some());
    let old = {
        let mut inner = lock(app);
        if inner.account == wanted {
            return;
        }
        let old = inner.account.take();
        *inner = Inner {
            account: wanted.clone(),
            unseen: 0,
        };
        old
    };
    if let Some(old) = old {
        crate::hermes::remove_user(app, &old, Kind::Onsite);
    }
    emit_unseen(app);
    let Some(account) = wanted else { return };
    crate::hermes::add_user(app, &account, Kind::Onsite);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Ok(c) = GqlCall::new(&app, &account) else {
            return;
        };
        match api::unseen(&c.http, c.auth()).await {
            Ok(n) => {
                let mut inner = lock(&app);
                if inner.account.as_deref() != Some(account.as_str()) {
                    return;
                }
                inner.unseen = n;
            }
            Err(e) => {
                eprintln!("bell: {e}");
                return;
            }
        }
        emit_unseen(&app);
    });
}

pub fn state(app: &AppHandle) -> BellState {
    let inner = lock(app);
    BellState {
        available: inner.account.is_some(),
        unseen: inner.unseen,
    }
}

fn emit_unseen(app: &AppHandle) {
    let _ = app.emit(UNSEEN, state(app));
}

pub fn on_event(app: &AppHandle, e: &Event) {
    if lock(app).account.as_deref() != Some(e.topic.channel_id.as_str()) {
        return;
    }
    let Some(live) = api::from_hermes(&e.payload) else {
        return;
    };
    match live {
        Live::Unseen(n) => {
            lock(app).unseen = n;
            emit_unseen(app);
        }
        Live::Created {
            item,
            toast,
            list,
            creator,
        } => {
            let n: Notifications = app.state::<Settings>().read(Partition::Notifications);
            if toast && n.bell.toast && !toasts::app_in_front(app) {
                toasts::show(
                    app,
                    Pop {
                        title: unmark(&item.body),
                        text: String::new(),
                        avatar: item.thumbnail.clone(),
                        open: Open::Link {
                            url: item
                                .url
                                .clone()
                                .unwrap_or_else(|| "https://www.twitch.tv/".into()),
                        },
                    },
                );
            }
            if list {
                let which = if creator { List::Creator } else { List::Viewer };
                let _ = app.emit(LIVE, json!({ "kind": "created", "list": which, "item": item }));
            }
        }
        Live::Updated(item) => {
            let _ = app.emit(LIVE, json!({ "kind": "updated", "item": item }));
        }
        Live::Deleted(id) => {
            let _ = app.emit(LIVE, json!({ "kind": "deleted", "ids": [id] }));
        }
        Live::Read(ids) => {
            let _ = app.emit(LIVE, json!({ "kind": "read", "ids": ids }));
        }
        Live::AllRead => {
            let _ = app.emit(LIVE, json!({ "kind": "allRead" }));
        }
    }
}

fn account(app: &AppHandle) -> Result<GqlCall, String> {
    let id = lock(app).account.clone().ok_or(NO_ACCOUNT)?;
    GqlCall::new(app, &id).map_err(|_| NO_ACCOUNT.to_owned())
}

fn failed(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        "Couldn't reach Twitch.".into()
    } else {
        eprintln!("bell: {e}");
        "Twitch didn't answer. Try again in a bit.".into()
    }
}

pub async fn list(app: &AppHandle, which: List, cursor: Option<String>) -> Result<BellPage, String> {
    let c = account(app)?;
    let page = api::list(&c.http, c.auth(), which, cursor.as_deref())
        .await
        .map_err(failed)?;
    Ok(BellPage {
        items: page.items,
        cursor: page.cursor,
    })
}

/// The bell was opened: the badge clears.
pub async fn seen(app: &AppHandle) -> Result<(), String> {
    let c = account(app)?;
    api::seen(&c.http, c.auth()).await.map_err(failed)?;
    lock(app).unseen = 0;
    emit_unseen(app);
    Ok(())
}

pub async fn read(app: &AppHandle, ids: Vec<String>) -> Result<(), String> {
    let c = account(app)?;
    api::read(&c.http, c.auth(), &ids).await.map_err(failed)
}

pub async fn read_all(app: &AppHandle, which: List) -> Result<(), String> {
    let id = lock(app).account.clone().ok_or(NO_ACCOUNT)?;
    let c = account(app)?;
    api::read_all(&c.http, c.auth(), which, &id).await.map_err(failed)
}

pub async fn delete(app: &AppHandle, id: String) -> Result<(), String> {
    let c = account(app)?;
    api::delete(&c.http, c.auth(), &id).await.map_err(failed)
}

/// Twitch's markdown without the marks: `**xqc** went live` → `xqc went live`.
pub fn unmark(md: &str) -> String {
    md.replace("**", "").replace("__", "").replace('\\', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_go() {
        assert_eq!(unmark("**xqc** went live: __Chess__"), "xqc went live: Chess");
    }
}
