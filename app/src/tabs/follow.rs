//! 1.11 Follow and unfollow. Helix can't anymore, so this needs the account's GQL token.

use cyex_twitch::gql;
use tauri::{AppHandle, Manager};

use crate::auth::{Auth, gql_token};

/// `None` when it can't be told: no GQL sign-in, or the channel is gone.
pub async fn following(app: &AppHandle, channel_id: &str, account: &str) -> Result<Option<bool>, String> {
    let Some((token, client_id)) = gql_token(app, account) else { return Ok(None) };
    let http = app.state::<Auth>().http.clone();
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    gql::is_following(&http, auth, channel_id).await.map_err(error_text)
}

pub async fn set(app: &AppHandle, channel_id: &str, account: &str, follow: bool) -> Result<(), String> {
    let (token, client_id) =
        gql_token(app, account).ok_or("Following needs a GQL sign-in. Add it in Settings → Accounts.")?;
    let http = app.state::<Auth>().http.clone();
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let done = if follow {
        gql::follow(&http, auth, channel_id).await
    } else {
        gql::unfollow(&http, auth, channel_id).await
    };
    done.map_err(error_text)
}

fn error_text(e: cyex_twitch::Error) -> String {
    match e.api_message() {
        None => "Couldn't reach Twitch.".into(),
        Some(m) if m.contains("integrity") => {
            "Twitch wants a fresh GQL sign-in for this. Sign in again in Settings → Accounts.".into()
        }
        Some("TOO_MANY_FOLLOWS") => "You follow too many channels.".into(),
        Some("FORBIDDEN") | Some("FOLLOW_BLOCKED") => "Twitch didn't allow it.".into(),
        Some(code) => format!("Didn't work ({code})."),
    }
}
