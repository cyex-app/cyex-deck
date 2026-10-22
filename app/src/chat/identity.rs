//! 1.40 How the account shows up in chat: badge (everywhere or per channel), name color and
//! (V107-Mk) smiley style. Badges and smileys need the GQL sign-in; the color works with either.

use cyex_twitch::gql::smilies::{self, Smilies};
use cyex_twitch::gql::{self, BadgePick, ChatIdentity, ColorRefusal};
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
const ANY_COLOR: &str = "Any color needs Prime or Turbo. Pick one of the named ones.";

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<Auth>().http.clone()
}

/// The account's picks in `login`'s channel. Without a GQL sign-in only the color shows.
pub async fn identity(app: &AppHandle, account: &str, login: &str) -> Result<ChatIdentity, String> {
    let http = http(app);
    if let Some((token, client_id)) = gql_token(app, account) {
        let auth = crate::auth::gql_auth(&token, &client_id);
        let (me, smilies) = tokio::join!(gql::chat_identity(&http, auth, login), smilies::smilies(&http, auth, None));
        let mut me = me.map_err(|e| e.to_string())?;
        // A failed smiley read only hides that part.
        if let Ok(s) = smilies {
            crate::emotes::set_smiley_style(app, account, s.current);
            me.smilies = s;
        }
        return Ok(me);
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
        smilies: Smilies::default(),
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
        Some(id) => gql::pick_channel_badge(&http, auth, None, id, badge).await,
        None => gql::pick_global_badge(&http, auth, None, badge).await,
    }
    .map_err(|e| e.to_string())
}

/// `#RRGGBB` or a named color (`DodgerBlue`, `dodger_blue`), as `/color` takes it.
pub fn parse_color(input: &str) -> Option<String> {
    let s = input.trim();
    if s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(s.to_ascii_uppercase());
    }
    let key: String = s.chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>().to_ascii_lowercase();
    NAMED
        .iter()
        .find(|(name, _)| name.replace('_', "") == key)
        .map(|(_, hex)| (*hex).to_owned())
}

/// What a refused color says.
fn refusal_text(r: &ColorRefusal) -> String {
    match r {
        ColorRefusal::TurboOnly => ANY_COLOR.into(),
        ColorRefusal::Invalid => "Twitch didn’t take that color.".into(),
        ColorRefusal::Other(code) => format!("Twitch didn’t change the color ({code})."),
    }
}

/// V107-Mk Through GQL's `updateChatColorV2` with the GQL sign-in; Helix when there's none,
/// or when GQL itself fails (not when it says no).
pub async fn set_color(app: &AppHandle, account: &str, color: &str) -> Result<(), String> {
    let color = parse_color(color).ok_or("Use a color like #1E90FF or a name like DodgerBlue.")?;
    let http = http(app);
    let mut gql_error = None;
    if let Some((token, client_id)) = gql_token(app, account) {
        let auth = crate::auth::gql_auth(&token, &client_id);
        match gql::set_chat_color(&http, auth, None, &color).await {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(r)) => return Err(refusal_text(&r)),
            Err(e) => gql_error = Some(e.to_string()),
        }
    }
    let Some((token, client_id)) = helix_scoped(app, account, COLOR_SCOPE) else {
        return Err(gql_error.unwrap_or_else(|| "Sign in to change your color.".into()));
    };
    let named = NAMED.iter().find(|(_, hex)| *hex == color).map(|(name, _)| *name);
    let auth = crate::auth::helix_auth(&token, &client_id);
    helix::set_chat_color(&http, auth, account, named.unwrap_or(&color))
        .await
        .map_err(|e| match e.api_message() {
            Some(m) if m.contains("Turbo or Prime") => ANY_COLOR.to_owned(),
            _ => e.to_string(),
        })
}

/// V107-Mk Picks the smiley set Twitch calls `name`, then reads it back; chat and the emote
/// menu switch to its art.
pub async fn set_smilies(app: &AppHandle, account: &str, name: &str) -> Result<Smilies, String> {
    let (token, client_id) =
        gql_token(app, account).ok_or("Smiley styles need a GQL sign-in. Add it in Settings → Accounts.")?;
    let auth = crate::auth::gql_auth(&token, &client_id);
    let http = http(app);
    smilies::set_smilies(&http, auth, None, account, name)
        .await
        .map_err(|e| e.to_string())?;
    let now = smilies::smilies(&http, auth, None).await.map_err(|e| e.to_string())?;
    crate::emotes::set_smiley_style(app, account, now.current);
    Ok(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_by_hex_or_name() {
        assert_eq!(parse_color(" #1e90ff ").as_deref(), Some("#1E90FF"));
        assert_eq!(parse_color("DodgerBlue").as_deref(), Some("#1E90FF"));
        assert_eq!(parse_color("dodger_blue").as_deref(), Some("#1E90FF"));
        assert_eq!(parse_color("Golden Rod").as_deref(), Some("#DAA520"));
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("purple"), None);
    }

    #[test]
    fn refusals_read_short() {
        assert_eq!(refusal_text(&ColorRefusal::TurboOnly), ANY_COLOR);
        assert_eq!(refusal_text(&ColorRefusal::Invalid), "Twitch didn’t take that color.");
    }
}
