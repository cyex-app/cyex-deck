//! F2, 1.45 Block list and `/block` for accounts with only a GQL sign-in. Shapes checked
//! against Twitch's schema without running the mutations (2026/10/10).

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Auth, query};
use crate::{Http, Result};

const LIST: &str = "query { currentUser { blockedUsers { id } } }";
const BLOCK: &str =
    "mutation($id: ID!) { blockUser(input: {targetUserID: $id, sourceContext: CHAT}) { targetUser { id } } }";
const UNBLOCK: &str = "mutation($id: ID!) { unblockUser(input: {targetUserID: $id}) { targetUser { id } } }";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    current_user: Option<Me>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Me {
    #[serde(default)]
    blocked_users: Vec<Option<Id>>,
}

#[derive(Deserialize)]
struct Id {
    id: String,
}

pub async fn blocked_ids(http: &Http, auth: Auth<'_>) -> Result<Vec<String>> {
    let data: Data = query(http, auth, LIST, json!({})).await?;
    Ok(data
        .current_user
        .map(|m| m.blocked_users.into_iter().flatten().map(|u| u.id).collect())
        .unwrap_or_default())
}

pub async fn block(http: &Http, auth: Auth<'_>, target_id: &str) -> Result<()> {
    query::<Value>(http, auth, BLOCK, json!({ "id": target_id }))
        .await
        .map(drop)
}

pub async fn unblock(http: &Http, auth: Auth<'_>, target_id: &str) -> Result<()> {
    query::<Value>(http, auth, UNBLOCK, json!({ "id": target_id }))
        .await
        .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_parses() {
        let d: Data =
            serde_json::from_str(r#"{"currentUser":{"blockedUsers":[{"id":"1"},null,{"id":"2"}]}}"#).unwrap();
        let ids: Vec<String> = d
            .current_user
            .unwrap()
            .blocked_users
            .into_iter()
            .flatten()
            .map(|u| u.id)
            .collect();
        assert_eq!(ids, ["1", "2"]);
    }
}
