//! 1.40 How the account shows up in chat: badge (everywhere or per channel), name color and
//! (V107-Mk) smiley style. Badges and smileys need the GQL sign-in; the color works with either.

use cyex_twitch::gql::smilies::{self, Smilies};
use cyex_twitch::gql::{self, BadgePick, ChatIdentity, ColorRefusal};
use cyex_twitch::helix;
use tauri::{AppHandle, Manager};

use crate::auth::{Auth, gql_token, helix_scoped};
// V107-H6b GQL failures in the stream manager's short words, not the raw error.
use crate::streammanager::words;

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

/// The GQL sign-in, or a stand-in token for the mock (the real one never goes there).
fn gql(app: &AppHandle, account: &str, base: Option<&str>) -> Option<(String, String)> {
    match base {
        Some(_) => Some(("mock-token".to_owned(), cyex_twitch::clients::WEB.to_owned())),
        None => gql_token(app, account),
    }
}

/// V107-Mx Test builds: badge reads and picks go to `tools/mock/identity.mjs`
/// (`CYEX_GQL_IDENTITY_MOCK=http://127.0.0.1:<port>`, test data folder only).
#[cfg(feature = "mocks")]
fn mock_base(app: &AppHandle) -> Result<Option<String>, String> {
    // Set but unusable: nothing is sent at all, never Twitch instead.
    crate::mock_url::gated(app, &["CYEX_GQL_IDENTITY_MOCK"], "identity")
}

#[cfg(not(feature = "mocks"))]
fn mock_base(_: &AppHandle) -> Result<Option<String>, String> {
    Ok(None)
}

/// The account's picks in `login`'s channel. Without a GQL sign-in only the color shows.
pub async fn identity(app: &AppHandle, account: &str, login: &str) -> Result<ChatIdentity, String> {
    let http = http(app);
    let base = mock_base(app)?;
    if let Some((token, client_id)) = gql(app, account, base.as_deref()) {
        let auth = crate::auth::gql_auth(&token, &client_id);
        let base = base.as_deref();
        let (me, smilies) =
            tokio::join!(gql::chat_identity(&http, auth, base, login), smilies::smilies(&http, auth, base));
        let mut me = me.map_err(|e| words::gql(&e))?;
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
    let color = gql::chat_color(&http, account).await.map_err(|e| words::gql(&e))?;
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
    let base = mock_base(app)?;
    let (token, client_id) =
        gql(app, account, base.as_deref()).ok_or("Badges need a GQL sign-in. Add it in Settings → Accounts.")?;
    let auth = crate::auth::gql_auth(&token, &client_id);
    let http = http(app);
    let base = base.as_deref();
    let taken = match channel_id {
        Some(id) => gql::pick_channel_badge(&http, auth, base, id, badge).await,
        None => gql::pick_global_badge(&http, auth, base, badge).await.map(|()| true),
    }
    .map_err(|e| words::gql(&e))?;
    taken.then_some(()).ok_or_else(|| "Twitch didn’t take that badge.".into())
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
        ColorRefusal::Other(code) => {
            eprintln!("chat color: refused with {code}");
            "Twitch didn’t change the color.".into()
        }
    }
}

/// V107-Mk What GQL's answer means: `Ok` is final (done, or Twitch said no, which Helix
/// would only repeat); `Err` is GQL itself failing, so Helix gets a go.
fn gql_verdict(r: cyex_twitch::Result<Result<(), ColorRefusal>>) -> Result<Result<(), String>, String> {
    match r {
        Ok(answer) => Ok(answer.map_err(|r| refusal_text(&r))),
        Err(e) => Err(words::gql(&e)),
    }
}

/// V107-Mk Through GQL's `updateChatColorV2` with the GQL sign-in; Helix when there's none,
/// or when GQL itself fails (not when it says no).
pub async fn set_color(app: &AppHandle, account: &str, color: &str) -> Result<(), String> {
    let color = parse_color(color).ok_or("Use a color like #1E90FF or a name like DodgerBlue.")?;
    let http = http(app);
    let base = mock_base(app)?;
    let mut gql_error = None;
    if let Some((token, client_id)) = gql(app, account, base.as_deref()) {
        let auth = crate::auth::gql_auth(&token, &client_id);
        match gql_verdict(gql::set_chat_color(&http, auth, base.as_deref(), &color).await) {
            Ok(done) => return done,
            // The mock never falls back to Twitch.
            Err(e) if base.is_some() => return Err(e),
            Err(e) => gql_error = Some(e),
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
    let base = mock_base(app)?;
    let (token, client_id) = gql(app, account, base.as_deref())
        .ok_or("Smiley styles need a GQL sign-in. Add it in Settings → Accounts.")?;
    let auth = crate::auth::gql_auth(&token, &client_id);
    let http = http(app);
    let base = base.as_deref();
    if !smilies::set_smilies(&http, auth, base, account, name)
        .await
        .map_err(|e| words::gql(&e))?
    {
        return Err("Twitch didn’t take that style.".into());
    }
    let now = smilies::smilies(&http, auth, base).await.map_err(|e| words::gql(&e))?;
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
        assert_eq!(refusal_text(&ColorRefusal::Other("X_Y".into())), "Twitch didn’t change the color.");
    }

    /// A Prime/Turbo refusal is final; only GQL failing itself falls back to Helix.
    #[test]
    fn helix_only_when_gql_fails() {
        assert_eq!(gql_verdict(Ok(Ok(()))), Ok(Ok(())));
        assert_eq!(gql_verdict(Ok(Err(ColorRefusal::TurboOnly))), Ok(Err(ANY_COLOR.to_owned())));
        assert_eq!(gql_verdict(Ok(Err(ColorRefusal::Invalid))).map(|r| r.is_err()), Ok(true));
        let down = cyex_twitch::Error::Api { status: 503, message: "down".into() };
        assert!(gql_verdict(Err(down)).is_err());
    }

    /// The popover's swatches (`identity.ts`) are Twitch's named colors here, in order.
    #[test]
    fn ui_presets_match() {
        const TS: &str = include_str!("../../../ui/src/features/chat/identity.ts");
        let hexes: Vec<&str> = TS
            .match_indices("'#")
            .filter_map(|(i, _)| TS.get(i + 1..i + 8))
            .filter(|h| h[1..].chars().all(|c| c.is_ascii_hexdigit()))
            .collect();
        assert_eq!(hexes, NAMED.map(|(_, hex)| hex));
    }
}
