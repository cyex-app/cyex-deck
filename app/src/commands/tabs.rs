//! Tab commands (spec 02).

use cyex_settings::schema::TabOp;
use cyex_twitch::gql::{self, ChannelInfo};
use tauri::{AppHandle, Manager};

use crate::auth::Auth;
use crate::tabs;

type CmdResult<T> = Result<T, String>;

/// Applies an edit to the tab list.
#[tauri::command]
pub fn tabs_apply(app: AppHandle, op: TabOp) -> CmdResult<tabs::TabsApplied> {
    tabs::apply(&app, op)
}

/// 1.14 Channels matching a name, user id or twitch.tv link.
#[tauri::command]
pub async fn tabs_lookup(app: AppHandle, input: String) -> CmdResult<Vec<ChannelInfo>> {
    let http = app.state::<Auth>().http.clone();
    gql::lookup(&http, &input).await.map_err(|e| {
        if e.is_network() {
            "Couldn't reach Twitch.".into()
        } else {
            e.to_string()
        }
    })
}

/// 1.29 The last live check for every tab channel. Updates arrive as `tabs:live`.
#[tauri::command]
pub fn tabs_live(app: AppHandle) -> Vec<ChannelInfo> {
    tabs::live(&app)
}

/// 1.11 `None` when it can't be told (no GQL sign-in).
#[tauri::command]
pub async fn channel_following(app: AppHandle, channel_id: String, account: String) -> CmdResult<Option<bool>> {
    tabs::follow::following(&app, &channel_id, &account).await
}

#[tauri::command]
pub async fn channel_follow(app: AppHandle, channel_id: String, account: String, follow: bool) -> CmdResult<()> {
    tabs::follow::set(&app, &channel_id, &account, follow).await
}
