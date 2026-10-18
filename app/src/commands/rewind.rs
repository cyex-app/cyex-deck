//! V1061-03 Twitch's Stream Rewind: the VOD this viewer may rewind in, if Twitch offers it.

use cyex_twitch::gql;
use tauri::{AppHandle, Manager};

use crate::auth::{Auth, gql_token};
#[cfg(feature = "mocks")]
use crate::player::mock;

/// The broadcast's VOD id when Twitch lets `account` rewind `login`'s stream. `None` for
/// everything else, errors too: no rewind shows nothing (Q20).
#[tauri::command]
pub async fn player_rewind(app: AppHandle, login: String, account: Option<String>) -> Option<String> {
    let http = app.state::<Auth>().http.clone();
    #[cfg(feature = "mocks")]
    if let Some(base) = mock::base(&app) {
        return mock::dvr(&http, &base, &login).await;
    }
    let (token, client_id) = gql_token(&app, account.as_deref()?)?;
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    gql::rewind::dvr_vod(&http, auth, &login).await.ok().flatten()
}
