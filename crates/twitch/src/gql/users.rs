use serde::Deserialize;
use serde_json::json;

use super::{Auth, query};
use crate::{Http, Result};

const USER_BY_ID: &str =
    "query($id: ID!) { user(id: $id) { id login displayName profileImageURL(width: 70) } }";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GqlUser {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(rename = "profileImageURL", default)]
    pub profile_image_url: Option<String>,
}

#[derive(Deserialize)]
struct Data {
    user: Option<GqlUser>,
}

/// `None` if the user doesn't exist or is banned.
pub async fn user_by_id(http: &Http, auth: Auth<'_>, id: &str) -> Result<Option<GqlUser>> {
    let data: Data = query(http, auth, USER_BY_ID, json!({ "id": id })).await?;
    Ok(data.user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_parses() {
        let d: Data = serde_json::from_str(
            r#"{"user":{"id":"12826","login":"twitch","displayName":"Twitch","profileImageURL":"https://x/p-70x70.png"}}"#,
        )
        .unwrap();
        let u = d.user.unwrap();
        assert_eq!(u.display_name, "Twitch");
        assert_eq!(u.profile_image_url.as_deref(), Some("https://x/p-70x70.png"));
    }
}
