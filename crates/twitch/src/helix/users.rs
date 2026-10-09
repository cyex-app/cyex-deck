use serde::Deserialize;

use super::{Auth, Data, get};
use crate::http::json;
use crate::{Http, Result};

#[derive(Clone, Debug, Deserialize)]
pub struct User {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(default)]
    pub profile_image_url: String,
}

/// Up to 100 ids per call.
pub async fn users_by_id(http: &Http, auth: Auth<'_>, ids: &[&str]) -> Result<Vec<User>> {
    let query: Vec<(&str, &str)> = ids.iter().take(100).map(|id| ("id", *id)).collect();
    let data: Data<User> = json(get(http, auth, "/users").query(&query)).await?;
    Ok(data.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn users_parse() {
        let d: Data<User> = serde_json::from_str(
            r#"{"data":[{"id":"1","login":"a","display_name":"A","type":"","broadcaster_type":"",
            "description":"","profile_image_url":"https://x/a-profile_image-300x300.png","offline_image_url":"",
            "view_count":0,"created_at":"2016-12-14T20:32:28Z"}]}"#,
        )
        .unwrap();
        assert_eq!(d.data[0].display_name, "A");
    }
}
