//! EventSub subscriptions over a websocket session (phase 10 feeds). The session itself
//! lives in `cyex_eventsub`; this creates and deletes subscriptions for it.

use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Auth, Data, post, request};
use crate::http::{empty, json};
use crate::{Http, Result};

#[derive(Deserialize)]
struct Created {
    id: String,
}

/// Subscribes the session to one type. Returns the subscription id. Twitch answers 409
/// when it exists already and 403 when the token lacks a scope or the role.
pub async fn subscribe(
    http: &Http,
    auth: Auth<'_>,
    session_id: &str,
    kind: &str,
    version: &str,
    condition: &Value,
) -> Result<String> {
    let req = post(http, auth, "/eventsub/subscriptions").json(&json!({
        "type": kind,
        "version": version,
        "condition": condition,
        "transport": { "method": "websocket", "session_id": session_id },
    }));
    let d: Data<Created> = json(req).await?;
    d.data.into_iter().next().map(|c| c.id).ok_or(crate::Error::Api {
        status: 202,
        message: "no subscription came back".into(),
    })
}

pub async fn unsubscribe(http: &Http, auth: Auth<'_>, id: &str) -> Result<()> {
    let req = request(http, auth, Method::DELETE, "/eventsub/subscriptions").query(&[("id", id)]);
    empty(req).await
}
