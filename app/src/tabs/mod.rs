//! Channel tabs (spec 02) wired to Tauri: edits from any window, the chat rooms tabs keep
//! joined (1.10), live dots and renamed channels (1.15, 1.29), following (1.11), reopening
//! closed tabs (U10), importing a Chatterino layout (V1041-02). The tab model and its rules
//! live in `cyex_settings::schema::tabs`.

mod closed;
pub mod follow;
pub mod import;
pub mod live;
pub mod mute;
pub mod viewers;
mod rooms;

pub use closed::any as reopenable;

pub use live::snapshot as live;

use cyex_settings::Partition;
use cyex_settings::schema::{TabOp, Tabs, random_id};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::settings::Settings;

pub fn init(app: &AppHandle) {
    app.manage(closed::ClosedTabs::default());
    app.manage(mute::Muted::default());
    import::init(app);
    rooms::sync(app);
    live::spawn(app);
}

/// The tabs partition changed, from any window, an import or a restored backup.
pub fn on_changed(app: &AppHandle) {
    rooms::sync(app);
    mute::prune(app);
    live::nudge(app);
    crate::miner::on_settings(app);
    crate::plugins::events::layout(app);
}

/// Joins and leaves rooms again, e.g. when the chats the miner sits in change.
pub fn sync_rooms(app: &AppHandle) {
    rooms::sync(app);
}

/// Accounts changed: a removed account's tabs now read anonymously.
pub fn on_accounts_changed(app: &AppHandle) {
    rooms::sync(app);
    crate::miner::on_settings(app);
}

/// What an edit made, and the list after it, so the window that asked can show it
/// right away (the `settings:changed` event follows).
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TabsApplied {
    /// The id of a new tab or group.
    pub made: Option<String>,
    pub tabs: Tabs,
}

pub fn apply(app: &AppHandle, op: TabOp) -> Result<TabsApplied, String> {
    let settings = app.state::<Settings>();
    let mut outcome = Ok(None);
    let mut closing = Vec::new();
    settings.update::<Tabs>(app, Partition::Tabs, |t| {
        closing = t.closing(&op);
        match t.apply(op, &mut random_id) {
            Ok(made) => {
                outcome = Ok(made);
                true
            }
            Err(e) => {
                outcome = Err(e.to_string());
                false
            }
        }
    })?;
    let made = outcome?;
    closed::push(app, closing);
    Ok(TabsApplied {
        made,
        tabs: settings.read(Partition::Tabs),
    })
}

/// U10 Brings back what the last close took, where it was, and shows it.
pub fn reopen(app: &AppHandle) -> Result<TabsApplied, String> {
    let settings = app.state::<Settings>();
    let mut made = None;
    if let Some(last) = closed::pop(app) {
        settings.update::<Tabs>(app, Partition::Tabs, |t| {
            made = t.restore(last, &mut random_id);
            made.is_some()
        })?;
    }
    Ok(TabsApplied {
        made,
        tabs: settings.read(Partition::Tabs),
    })
}
