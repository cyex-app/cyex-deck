//! V108-N16 `GET /moderation/moderated_channels`: the channels a user moderates. Scope
//! `user:read:moderated_channels`, and `user_id` has to be the token's own user.

use serde::Deserialize;

use super::{Auth, get, pages};
use crate::{Http, Result};

const MAX_PAGES: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ModChannel {
    pub broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
}

/// Every channel the user moderates, 100 per page.
pub async fn moderated_channels(http: &Http, auth: Auth<'_>, user_id: &str) -> Result<Vec<ModChannel>> {
    let (list, _) =
        pages(MAX_PAGES, || get(http, auth, "/moderation/moderated_channels").query(&[("user_id", user_id), ("first", "100")]))
            .await?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::Page;

    #[test]
    fn channels_parse_with_a_cursor() {
        let p: Page<ModChannel> = serde_json::from_str(
            r#"{"data":[{"broadcaster_id":"1","broadcaster_login":"a","broadcaster_name":"A"},
                        {"broadcaster_id":"2","broadcaster_login":"b","broadcaster_name":"B"}],
                "pagination":{"cursor":"abc"}}"#,
        )
        .unwrap();
        assert_eq!(p.data.len(), 2);
        assert_eq!(p.data[1].broadcaster_login, "b");
        assert_eq!(p.pagination.cursor.as_deref(), Some("abc"));
    }
}
