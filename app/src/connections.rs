//! Connection counter (1.13, 1.23, D7): joined chat rooms per account against Twitch's
//! limit. Chat (phase 3) updates the budgets; this module reports them.

use std::collections::HashMap;
use std::sync::Mutex;

use cyex_chat::{ConnectionCount, JoinBudget};
use tauri::{AppHandle, Emitter, Manager};

const CHANGED_EVENT: &str = "connections:changed";

/// Join budget per account, by user id.
#[derive(Default)]
pub struct Connections(pub Mutex<HashMap<String, JoinBudget>>);

pub fn init(app: &AppHandle) {
    app.manage(Connections::default());
}

/// One entry per signed-in account, zeros included.
pub fn counts(app: &AppHandle) -> Vec<ConnectionCount> {
    let budgets = app
        .state::<Connections>()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    crate::auth::accounts(app)
        .list
        .iter()
        .map(|a| {
            ConnectionCount::of(
                &a.user_id,
                budgets.get(&a.user_id).unwrap_or(&JoinBudget::default()),
            )
        })
        .collect()
}

pub fn notify(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, counts(app));
}
