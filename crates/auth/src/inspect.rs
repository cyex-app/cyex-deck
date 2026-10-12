//! Looks up who a token belongs to, for new sign-ins and pasted tokens.

use cyex_twitch::{Http, clients, gql, helix, id};

use crate::AuthError;

#[derive(Debug)]
pub struct Inspected {
    pub validation: id::Validation,
    pub display_name: String,
    pub avatar: String,
}

pub async fn inspect(http: &Http, access: &str) -> Result<Inspected, AuthError> {
    let validation = id::validate(http, access).await?;
    if validation.user_id.is_empty() {
        return Err(AuthError::AppToken);
    }
    let helix_auth = (!clients::is_first_party(&validation.client_id)).then(|| helix::Auth {
        client_id: &validation.client_id,
        token: access,
    });
    let (display_name, avatar) = profile(http, &validation.user_id, helix_auth)
        .await
        .unwrap_or_else(|| (validation.login.clone(), String::new()));
    Ok(Inspected {
        validation,
        display_name,
        avatar,
    })
}

/// Display name and small avatar. Helix with the account's own token if it has one,
/// otherwise public GQL data.
pub async fn profile(
    http: &Http,
    user_id: &str,
    helix_auth: Option<helix::Auth<'_>>,
) -> Option<(String, String)> {
    if let Some(auth) = helix_auth
        && let Ok(users) = helix::users_by_id(http, auth, &[user_id]).await
        && let Some(user) = users.into_iter().next()
    {
        return Some((user.display_name, small_avatar(&user.profile_image_url)));
    }
    let auth = gql::Auth {
        client_id: clients::WEB,
        token: None,
    };
    let user = gql::user_by_id(http, auth, user_id).await.ok()??;
    Some((user.display_name, user.profile_image_url.unwrap_or_default()))
}

/// Helix sends 300×300; the CDN also has 70×70, plenty for a 32 px avatar.
fn small_avatar(url: &str) -> String {
    url.replace("-300x300.", "-70x70.")
}

#[cfg(test)]
mod tests {
    use super::small_avatar;

    #[test]
    fn avatars_use_the_small_size() {
        assert_eq!(
            small_avatar("https://static-cdn.jtvnw.net/u/abc-profile_image-300x300.png"),
            "https://static-cdn.jtvnw.net/u/abc-profile_image-70x70.png"
        );
    }
}
