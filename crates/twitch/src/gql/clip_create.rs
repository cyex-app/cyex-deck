//! P9, V102-08 Making a clip with a GQL sign-in, the way Twitch's player does it:
//! `createClip` on the live broadcast, at its live edge. The clip covers the seconds
//! before that point. Schema-checked 2026/10/11: `CreateClipInput { broadcasterID,
//! offsetSeconds, broadcastID, videoID }`.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Auth, query};
use crate::{Error, Http, Result};

/// The channel's live broadcast: its id and when it started (RFC 3339). `None` offline.
pub async fn live_broadcast(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
) -> Result<Option<(String, String)>> {
    const Q: &str = "query($id: ID!) { user(id: $id) { stream { id createdAt } } }";
    let data: StreamData = query(http, auth, Q, json!({ "id": channel_id })).await?;
    Ok(data.user.and_then(|u| u.stream).map(|s| (s.id, s.created_at)))
}

/// Clips `broadcast_id` up to `offset_secs` into it. The new clip's slug; [`Error::Refused`]
/// with Twitch's code when it says no (e.g. `CHANNEL_NOT_LIVE`, `REQUEST_THROTTLED`).
pub async fn create_clip(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    broadcast_id: &str,
    offset_secs: f64,
) -> Result<String> {
    const Q: &str = "mutation($input: CreateClipInput!) {
      createClip(input: $input) { clip { slug } error { code } }
    }";
    let vars = json!({ "input": {
        "broadcasterID": channel_id,
        "broadcastID": broadcast_id,
        "offsetSeconds": offset_secs.max(0.0),
    } });
    let data: Value = query(http, auth, Q, vars).await?;
    slug_of(&data)
}

fn slug_of(data: &Value) -> Result<String> {
    let made = &data["createClip"];
    if let Some(code) = made["error"]["code"].as_str() {
        return Err(Error::Refused(code.to_owned()));
    }
    made["clip"]["slug"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| Error::Refused("UNKNOWN".into()))
}

#[derive(Deserialize)]
struct StreamData {
    user: Option<StreamUser>,
}

#[derive(Deserialize)]
struct StreamUser {
    stream: Option<Stream>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stream {
    id: String,
    created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_or_twitchs_code() {
        let ok = json!({ "createClip": { "clip": { "slug": "Fresh-AbC" }, "error": null } });
        assert_eq!(slug_of(&ok).unwrap(), "Fresh-AbC");
        let offline = json!({ "createClip": { "clip": null, "error": { "code": "CHANNEL_NOT_LIVE" } } });
        assert!(matches!(slug_of(&offline), Err(Error::Refused(c)) if c == "CHANNEL_NOT_LIVE"));
        let empty = json!({ "createClip": null });
        assert!(matches!(slug_of(&empty), Err(Error::Refused(c)) if c == "UNKNOWN"));
    }

    #[test]
    fn offline_channels_have_no_broadcast() {
        let d: StreamData = serde_json::from_str(r#"{"user":{"stream":null}}"#).unwrap();
        assert!(d.user.and_then(|u| u.stream).is_none());
        let d: StreamData =
            serde_json::from_str(r#"{"user":{"stream":{"id":"42","createdAt":"2026-10-11T08:00:00Z"}}}"#)
                .unwrap();
        assert_eq!(d.user.unwrap().stream.unwrap().id, "42");
    }
}
