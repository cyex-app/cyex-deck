//! V107-M8 Someone's mod and VIP status in a channel, for the role buttons: read from the
//! badges Twitch says they have there (public, no sign-in), and told to every window as
//! `mod:standing` when a change goes through, ours or another mod's.

use cyex_moderation::roles::{Role, holds_of};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use super::http;

/// One role changed for one user in one channel. Granting one clears the other.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleChange {
    /// The channel's login.
    pub channel: String,
    pub login: String,
    pub moderator: bool,
    pub on: bool,
}

/// What `login` holds in `channel` now; `None` when Twitch can't say.
pub async fn holds(app: &AppHandle, channel: &str, login: &str) -> Option<Vec<Role>> {
    let badges = cyex_twitch::gql::viewer::earned_badges(&http(app), login, channel).await.ok()?;
    Some(holds_of(badges.iter().map(|b| b.set.as_str())))
}

pub fn changed(app: &AppHandle, channel: &str, login: &str, moderator: bool, on: bool) {
    let change = RoleChange {
        channel: channel.to_ascii_lowercase(),
        login: login.to_ascii_lowercase(),
        moderator,
        on,
    };
    let _ = app.emit("mod:standing", change);
}
