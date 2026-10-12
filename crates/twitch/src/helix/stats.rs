//! 1.35 Live numbers for the stream manager: the current stream (viewers, uptime), the
//! follower total (`moderator:read:followers`) and the subscriber total and sub points
//! (`channel:read:subscriptions`). Each is one small request, polled while the tab is open.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Auth, Data, Page, get};
use crate::http::json;
use crate::{Http, Result};

/// The stream, when it's live.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export, rename = "HelixStream")]
pub struct Stream {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub game_name: String,
    #[serde(rename(deserialize = "viewer_count"), default)]
    #[ts(type = "number")]
    pub viewers: u64,
    /// RFC 3339 start.
    #[serde(default)]
    pub started_at: String,
    /// `live` or `rerun`.
    #[serde(rename(deserialize = "type"), default)]
    pub kind: String,
}

/// The stream now, or `None` when offline.
pub async fn stream(http: &Http, auth: Auth<'_>, user_id: &str) -> Result<Option<Stream>> {
    let req = get(http, auth, "/streams").query(&[("user_id", user_id), ("first", "1")]);
    let d: Data<Stream> = json(req).await?;
    Ok(d.data.into_iter().next())
}

/// How many follow the channel.
pub async fn followers_total(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<u64> {
    let req =
        get(http, auth, "/channels/followers").query(&[("broadcaster_id", broadcaster_id), ("first", "1")]);
    let p: Page<serde_json::Value> = json(req).await?;
    Ok(p.total.unwrap_or(0))
}

/// Subscriber total and the sub points a channel has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SubTotals")]
pub struct Subs {
    #[ts(type = "number")]
    pub total: u64,
    #[ts(type = "number")]
    pub points: u64,
}

#[derive(Deserialize)]
struct SubsPage {
    #[serde(default)]
    total: u64,
    #[serde(default)]
    points: u64,
}

/// The subscriber count and sub points (`channel:read:subscriptions`). The broadcaster's
/// own subscription, if any, counts as 2 points, like Twitch's dashboard.
pub async fn subscriptions(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<Subs> {
    let req = get(http, auth, "/subscriptions").query(&[("broadcaster_id", broadcaster_id), ("first", "1")]);
    let p: SubsPage = json(req).await?;
    Ok(Subs {
        total: p.total,
        points: p.points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_parses_or_empty() {
        let d: Data<Stream> = serde_json::from_str(
            r#"{"data":[{"id":"1","user_id":"2","user_name":"B","game_name":"Chess","type":"live",
            "title":"blitz","viewer_count":4321,"started_at":"2024-01-01T10:00:00Z"}],"pagination":{}}"#,
        )
        .unwrap();
        let s = d.data.into_iter().next().unwrap();
        assert_eq!(
            (s.viewers, s.game_name.as_str(), s.kind.as_str()),
            (4321, "Chess", "live")
        );

        let off: Data<Stream> = serde_json::from_str(r#"{"data":[],"pagination":{}}"#).unwrap();
        assert!(off.data.into_iter().next().is_none());
    }

    #[test]
    fn subs_total_and_points() {
        let p: SubsPage = serde_json::from_str(
            r#"{"data":[{"broadcaster_id":"1","tier":"1000","user_id":"9"}],"total":512,"points":640,"pagination":{}}"#,
        )
        .unwrap();
        assert_eq!((p.total, p.points), (512, 640));
    }
}
