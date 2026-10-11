//! 1.35 Raids from your own channel: start the countdown or call it off. GQL has no read
//! of a pending raid; its state comes live from Hermes `raid.<channel id>`.

use serde_json::json;

use super::mutate;
use crate::gql::Auth;
use crate::{Http, Result};

const CREATE: &str = "mutation($input: CreateRaidInput!) { createRaid(input: $input) { error { code } } }";
const CANCEL: &str = "mutation($input: CancelRaidInput!) { cancelRaid(input: $input) { error { code } } }";

pub async fn start(http: &Http, auth: Auth<'_>, channel_id: &str, target_id: &str) -> Result<()> {
    let input = json!({ "sourceID": channel_id, "targetID": target_id });
    mutate(http, auth, CREATE, "createRaid", input).await
}

pub async fn cancel(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<()> {
    mutate(
        http,
        auth,
        CANCEL,
        "cancelRaid",
        json!({ "sourceID": channel_id }),
    )
    .await
}
