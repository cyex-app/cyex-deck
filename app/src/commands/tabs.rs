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

/// U10 Reopens what the last close took. `made` is the tab it shows, if any.
#[tauri::command]
pub fn tabs_reopen(app: AppHandle) -> CmdResult<tabs::TabsApplied> {
    tabs::reopen(&app)
}

/// U10 Whether something can be reopened. Changes arrive as `tabs:reopenable`.
#[tauri::command]
pub fn tabs_reopenable(app: AppHandle) -> bool {
    tabs::reopenable(&app)
}

/// U9 Chats with news nobody has seen. Changes arrive as `chat:unread`.
#[tauri::command]
pub fn tabs_unread(app: AppHandle) -> Vec<crate::chat::unread::RoomUnread> {
    crate::chat::unread::snapshot(&app)
}

/// 1.14 Channels matching a name, user id or twitch.tv link.
#[tauri::command]
pub async fn tabs_lookup(app: AppHandle, input: String) -> CmdResult<Vec<ChannelInfo>> {
    let http = app.state::<Auth>().http.clone();
    gql::lookup(&http, &input).await.map_err(|e| {
        if e.is_network() {
            "Couldn’t reach Twitch.".into()
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

/// V106-18 Viewer counts of a live tab channel since the app saw it start (memory only).
#[tauri::command]
pub fn tabs_viewers(app: AppHandle, channel_id: String) -> Option<tabs::viewers::ViewerHistory> {
    tabs::viewers::get(&app, &channel_id)
}

/// V106-25 Mutes or unmutes everything in a tab (not saved). Returns the muted tabs; changes
/// arrive as `tabs:muted`.
#[tauri::command]
pub fn tabs_mute(app: AppHandle, tab: String, on: bool) -> Vec<String> {
    tabs::mute::set(&app, &tab, on)
}

/// V106-25 Tabs muted from their right-click menu.
#[tauri::command]
pub fn tabs_muted(app: AppHandle) -> Vec<String> {
    tabs::mute::list(&app)
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
