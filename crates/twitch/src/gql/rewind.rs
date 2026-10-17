//! V1061-03 Twitch's Stream Rewind gate. `channel.self.dvr.vodID` is set only when Twitch
//! lets this viewer rewind this channel's stream (Turbo or an ad-free sub, rewind on).
//! Raw document rebuilt from twitch.tv's `DVRVideoIDQuery` (research-stream-rewind.md s.3).

use serde::Deserialize;
use serde_json::json;

use super::{Auth, query};
use crate::{Http, Result};

const DVR: &str =
    "query($login: String) { user(login: $login) { id channel { id self { id dvr { vodID } } } } }";

#[derive(Deserialize)]
struct Data {
    user: Option<User>,
}

#[derive(Deserialize)]
struct User {
    channel: Option<Channel>,
}

#[derive(Deserialize)]
struct Channel {
    #[serde(rename = "self")]
    me: Option<Me>,
}

#[derive(Deserialize)]
struct Me {
    dvr: Option<Dvr>,
}

#[derive(Deserialize)]
struct Dvr {
    #[serde(rename = "vodID")]
    vod_id: Option<String>,
}

/// The VOD this viewer may rewind in; `None` when Twitch doesn't offer it.
pub async fn dvr_vod(http: &Http, auth: Auth<'_>, login: &str) -> Result<Option<String>> {
    let data: Data = query(http, auth, DVR, json!({ "login": login })).await?;
    Ok(vod_id(data))
}

fn vod_id(d: Data) -> Option<String> {
    d.user?.channel?.me?.dvr?.vod_id.filter(|id| !id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Option<String> {
        vod_id(serde_json::from_str(s).unwrap())
    }

    #[test]
    fn dvr_parses() {
        let ok = r#"{"user":{"id":"1","channel":{"id":"1","self":{"id":"x","dvr":{"vodID":"2903273448"}}}}}"#;
        assert_eq!(parse(ok).as_deref(), Some("2903273448"));
        assert_eq!(
            parse(r#"{"user":{"id":"1","channel":{"id":"1","self":{"id":"x","dvr":null}}}}"#),
            None
        );
        assert_eq!(
            parse(r#"{"user":{"id":"1","channel":{"id":"1","self":null}}}"#),
            None
        );
        assert_eq!(parse(r#"{"user":null}"#), None);
        assert_eq!(
            parse(r#"{"user":{"id":"1","channel":{"id":"1","self":{"id":"x","dvr":{"vodID":""}}}}}"#),
            None
        );
    }
}
