//! Home and discover commands (1.6, 1.7).

use cyex_twitch::gql::{self, Category, CategoryStreams, ChannelCard, Page, SearchResults, StreamFilter};
use tauri::{AppHandle, Manager};

use crate::auth::Auth;
use crate::browse::{discover, follows};

type CmdResult<T> = Result<T, String>;

/// 1.6 Channels the account follows. `force` fetches the whole list again.
#[tauri::command]
pub async fn home_follows(app: AppHandle, account: String, force: bool) -> CmdResult<Vec<ChannelCard>> {
    follows::follows(&app, &account, force).await
}

/// H2 Channels by user id, e.g. favorites. Public.
#[tauri::command]
pub async fn home_channels(app: AppHandle, ids: Vec<String>) -> CmdResult<Vec<ChannelCard>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let http = app.state::<Auth>().http.clone();
    gql::cards(&http, &ids).await.map_err(follows::error_text)
}

#[tauri::command]
pub async fn discover_streams(
    app: AppHandle,
    filter: StreamFilter,
    after: Option<String>,
) -> CmdResult<Page<ChannelCard>> {
    discover::streams(&app, &filter, after.as_deref()).await
}

#[tauri::command]
pub async fn discover_categories(app: AppHandle, after: Option<String>) -> CmdResult<Page<Category>> {
    discover::categories(&app, after.as_deref()).await
}

#[tauri::command]
pub async fn discover_category(
    app: AppHandle,
    slug: String,
    filter: StreamFilter,
    after: Option<String>,
) -> CmdResult<Option<CategoryStreams>> {
    discover::category(&app, &slug, &filter, after.as_deref()).await
}

#[tauri::command]
pub async fn discover_search(app: AppHandle, text: String) -> CmdResult<SearchResults> {
    discover::search(&app, text.trim()).await
}

#[tauri::command]
pub async fn discover_search_more(
    app: AppHandle,
    text: String,
    after: String,
) -> CmdResult<Page<ChannelCard>> {
    discover::search_more(&app, text.trim(), &after).await
}

/// H4
#[tauri::command]
pub async fn discover_followed_categories(app: AppHandle, account: String) -> CmdResult<Vec<Category>> {
    discover::followed_categories(&app, &account).await
}
