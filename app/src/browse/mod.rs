//! Home and discover (1.6, 1.7) wired to Tauri. Follows are cached per account
//! (`follows`); discover lists go straight to Twitch (`discover`).

pub mod discover;
pub mod follows;
mod ics;
pub mod schedule;

use tauri::{AppHandle, Manager};

pub fn init(app: &AppHandle) {
    app.manage(follows::Follows::default());
    app.manage(schedule::Schedules::default());
}
