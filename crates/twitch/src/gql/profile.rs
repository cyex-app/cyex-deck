//! 2.2 The account's Twitch profile: display name (capitalization only), bio, chat color.
//! GQL with the account's token; shapes checked 2026/10/10, the update never run.

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, query};
use crate::{Error, Http, Result};

/// Twitch's limit for the bio.
pub const BIO_MAX: usize = 300;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TwitchProfile")]
pub struct Profile {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub bio: String,
    /// `#rrggbb`, or empty when never set.
    pub color: String,
    /// Prime or Turbo: any color, not just Twitch's named ones.
    pub any_color: bool,
    /// Twitch allows a non-Latin display name for these languages.
    pub language: String,
}

pub async fn profile(http: &Http, auth: Auth<'_>) -> Result<Profile> {
    let q = "query { currentUser { id login displayName description chatColor hasPrime hasTurbo \
        settings { preferredLanguageTag } } }";
    let data: Data = query(http, auth, q, json!({})).await?;
    let u = data.current_user.ok_or_else(|| api("unauthenticated"))?;
    Ok(Profile {
        id: u.id,
        login: u.login,
        display_name: u.display_name,
        bio: u.description.unwrap_or_default(),
        color: u.chat_color.unwrap_or_default(),
        any_color: u.has_prime || u.has_turbo,
        language: u
            .settings
            .and_then(|s| s.preferred_language_tag)
            .unwrap_or_default(),
    })
}

/// Saves display name and bio together, as twitch.tv does. Errors carry Twitch's code,
/// e.g. `DISPLAY_NAME_ONLY_CAP` or `DESCRIPTION_TOO_LONG`.
pub async fn update(http: &Http, auth: Auth<'_>, user_id: &str, display_name: &str, bio: &str) -> Result<()> {
    let q = "mutation($id: ID!, $name: String, $bio: String) { \
        updateUser(input: {userID: $id, displayName: $name, description: $bio}) { \
        user { id } error { code } } }";
    let vars = json!({ "id": user_id, "name": display_name, "bio": bio });
    let data: UpdateData = query(http, auth, q, vars).await?;
    match data.update_user.and_then(|u| u.error) {
        Some(e) => Err(api(&e.code)),
        None => Ok(()),
    }
}

/// Twitch only lets the display name differ from the login in capitalization, unless
/// the account's language is Chinese, Japanese or Korean.
pub fn name_allowed(login: &str, name: &str, language: &str) -> bool {
    let cjk = ["zh", "ja", "ko"].iter().any(|l| language.starts_with(l));
    !name.trim().is_empty() && (name.eq_ignore_ascii_case(login) || cjk)
}

fn api(code: &str) -> Error {
    Error::Api {
        status: 200,
        message: code.to_owned(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    current_user: Option<User>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: String,
    login: String,
    #[serde(default)]
    display_name: String,
    description: Option<String>,
    chat_color: Option<String>,
    #[serde(default)]
    has_prime: bool,
    #[serde(default)]
    has_turbo: bool,
    settings: Option<UserSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserSettings {
    preferred_language_tag: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateData {
    update_user: Option<UpdatePayload>,
}

#[derive(Deserialize)]
struct UpdatePayload {
    error: Option<Code>,
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(name_allowed("cyexbot", "CyexBot", "en"));
        assert!(!name_allowed("cyexbot", "cyex_bot", "en"));
        assert!(name_allowed("cyexbot", "サイエックス", "ja-JP"));
        assert!(!name_allowed("cyexbot", " ", "ja"));
    }

    #[test]
    fn parses() {
        let d: Data = serde_json::from_str(
            r##"{"currentUser":{"id":"1","login":"a","displayName":"A","description":null,"chatColor":"#FF0000",
              "hasPrime":true,"hasTurbo":false,"settings":{"preferredLanguageTag":"EN"}}}"##,
        )
        .unwrap();
        assert!(d.current_user.unwrap().has_prime);
        let e: UpdateData =
            serde_json::from_str(r#"{"updateUser":{"user":null,"error":{"code":"DISPLAY_NAME_ONLY_CAP"}}}"#)
                .unwrap();
        assert_eq!(
            e.update_user.unwrap().error.unwrap().code,
            "DISPLAY_NAME_ONLY_CAP"
        );
    }
}
