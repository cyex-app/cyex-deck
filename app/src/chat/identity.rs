//! 1.40 How the account shows up in chat: badge (everywhere or per channel) and name
//! color. Badges need the GQL sign-in; the color works with either.

use cyex_twitch::gql::{self, BadgePick, ChatIdentity};
use cyex_twitch::helix;
use tauri::{AppHandle, Manager};

use crate::auth::{Auth, gql_token, helix_scoped};

/// Twitch's named colors, the ones anyone may pick.
pub const NAMED: [(&str, &str); 15] = [
    ("blue", "#0000FF"),
    ("blue_violet", "#8A2BE2"),
    ("cadet_blue", "#5F9EA0"),
    ("chocolate", "#D2691E"),
    ("coral", "#FF7F50"),
    ("dodger_blue", "#1E90FF"),
    ("firebrick", "#B22222"),
    ("golden_rod", "#DAA520"),
    ("green", "#008000"),
    ("hot_pink", "#FF69B4"),
    ("orange_red", "#FF4500"),
    ("red", "#FF0000"),
    ("sea_green", "#2E8B57"),
    ("spring_green", "#00FF7F"),
    ("yellow_green", "#9ACD32"),
];

const COLOR_SCOPE: &str = "user:manage:chat_color";

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<Auth>().http.clone()
}

/// The account's picks in `login`'s channel. Without a GQL sign-in only the color shows.
pub async fn identity(app: &AppHandle, account: &str, login: &str) -> Result<ChatIdentity, String> {
    let http = http(app);
    if let Some((token, client_id)) = gql_token(app, account) {
        let auth = crate::auth::gql_auth(&token, &client_id);
        return gql::chat_identity(&http, auth, login)
            .await
            .map_err(|e| e.to_string());
    }
    if helix_scoped(app, account, COLOR_SCOPE).is_none() {
        return Err("Sign in to change how you show up in chat.".into());
    }
    let color = gql::chat_color(&http, account).await.map_err(|e| e.to_string())?;
    Ok(ChatIdentity {
        color,
        any_color: false,
        global: BadgePick::default(),
        channel: BadgePick::default(),
        channel_id: None,
    })
}

/// `channel_id` `None` picks the global badge. `badge` `None` takes it off.
pub async fn pick_badge(
    app: &AppHandle,
    account: &str,
    channel_id: Option<&str>,
    badge: Option<(&str, &str)>,
) -> Result<(), String> {
    let (token, client_id) =
        gql_token(app, account).ok_or("Badges need a GQL sign-in. Add it in Settings → Accounts.")?;
    let auth = crate::auth::gql_auth(&token, &client_id);
    let http = http(app);
    match channel_id {
        Some(id) => gql::pick_channel_badge(&http, auth, id, badge).await,
        None => gql::pick_global_badge(&http, auth, badge).await,
    }
    .map_err(|e| e.to_string())
}

/// `#RRGGBB`. Through Helix when allowed (named colors go by name), else GQL.
pub async fn set_color(app: &AppHandle, account: &str, color: &str) -> Result<(), String> {
    let color = color.trim().to_ascii_uppercase();
    if !(color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())) {
        return Err("Use a color like #1E90FF.".into());
    }
    let http = http(app);
    let result = if let Some((token, client_id)) = helix_scoped(app, account, COLOR_SCOPE) {
        let named = NAMED.iter().find(|(_, hex)| *hex == color).map(|(name, _)| *name);
        let auth = crate::auth::helix_auth(&token, &client_id);
        helix::set_chat_color(&http, auth, account, named.unwrap_or(&color)).await
    } else if let Some((token, client_id)) = gql_token(app, account) {
        let auth = crate::auth::gql_auth(&token, &client_id);
        gql::set_chat_color(&http, auth, &color).await
    } else {
        return Err("Sign in to change your color.".into());
    };
    result.map_err(|e| match e.api_message() {
        Some(m) if m.contains("Turbo or Prime") => {
            "Any color needs Prime or Turbo. Pick one of the named ones.".to_owned()
        }
        _ => e.to_string(),
    })
}
