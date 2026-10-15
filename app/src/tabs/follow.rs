//! 1.11 Follow and unfollow. Helix can't anymore, so this needs the account's GQL token.

use cyex_twitch::gql;
use tauri::{AppHandle, Manager};

use crate::auth::{Auth, gql_token};

/// `None` when it can't be told: no GQL sign-in, or the channel is gone.
pub async fn following(app: &AppHandle, channel_id: &str, account: &str) -> Result<Option<bool>, String> {
    let Some((token, client_id)) = gql_token(app, account) else {
        return Ok(None);
    };
    let http = app.state::<Auth>().http.clone();
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    gql::is_following(&http, auth, channel_id)
        .await
        .map_err(|e| error_text(&e))
}

pub async fn set(app: &AppHandle, channel_id: &str, account: &str, follow: bool) -> Result<(), String> {
    let (token, client_id) =
        gql_token(app, account).ok_or("It needs a GQL sign-in. Add it in Settings → Accounts.")?;
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
    done.map_err(|e| error_text(&e))
}

/// V105-07 A sentence that reads after "Couldn't follow: ".
fn error_text(e: &cyex_twitch::Error) -> String {
    match e.api_message() {
        None => "Twitch didn’t answer.".into(),
        Some(m) if m.contains("integrity") => {
            "Twitch wants a fresh GQL sign-in. Sign in again in Settings → Accounts.".into()
        }
        Some("TOO_MANY_FOLLOWS") => "You follow too many channels.".into(),
        Some("FORBIDDEN") | Some("FOLLOW_BLOCKED") => "Twitch didn’t allow it.".into(),
        Some(code) => format!("Twitch said {code}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api(message: &str) -> cyex_twitch::Error {
        cyex_twitch::Error::Api {
            status: 200,
            message: message.into(),
        }
    }

    #[test]
    fn reasons_read_well_after_couldnt_follow() {
        assert_eq!(
            error_text(&api("TOO_MANY_FOLLOWS")),
            "You follow too many channels."
        );
        assert_eq!(error_text(&api("FOLLOW_BLOCKED")), "Twitch didn’t allow it.");
        assert_eq!(error_text(&api("FORBIDDEN")), "Twitch didn’t allow it.");
        assert!(error_text(&api("integrity check failed")).starts_with("Twitch wants a fresh GQL sign-in."));
        assert_eq!(error_text(&api("WHATEVER")), "Twitch said WHATEVER.");
    }

    #[test]
    fn other_errors_are_no_answer() {
        let decode = cyex_twitch::Error::Decode(serde_json::from_str::<u8>("x").unwrap_err());
        assert_eq!(error_text(&decode), "Twitch didn’t answer.");
    }
}
