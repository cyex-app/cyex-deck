//! 1.35 The numbers on the stream manager: live, viewers, uptime, title, category and
//! followers (public), plus subscribers and sub points (the broadcaster's own read, which
//! errors for channels without subs, so it runs as its own query in the same request).

use serde::Deserialize;
use serde_json::{Value, json};

use crate::gql::{Auth, batch};
use crate::{Error, Http, Result};

const PUBLIC: &str = "query($id: ID!) { user(id: $id) { broadcastSettings { title game { displayName } } \
    stream { createdAt viewersCount } followers { totalCount } } }";
const SUBS: &str = "query($id: ID!) { user(id: $id) { subscriberScore { count current } } }";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Numbers {
    pub live: bool,
    pub title: String,
    pub game: String,
    pub viewers: u64,
    /// RFC 3339; empty while offline.
    pub started_at: String,
    pub followers: Option<u64>,
    /// (subscribers, sub points); `None` when Twitch has none for the channel.
    pub subs: Option<(u64, u64)>,
}

#[derive(Deserialize)]
struct PublicData {
    user: Option<PublicUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicUser {
    broadcast_settings: Option<Settings>,
    stream: Option<Stream>,
    followers: Option<Total>,
}

#[derive(Deserialize)]
struct Settings {
    title: Option<String>,
    game: Option<Game>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Game {
    display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stream {
    created_at: Option<String>,
    viewers_count: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Total {
    total_count: Option<u64>,
}

#[derive(Deserialize)]
struct SubsData {
    user: Option<SubsUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubsUser {
    subscriber_score: Option<Score>,
}

#[derive(Deserialize)]
struct Score {
    count: Option<u64>,
    current: Option<u64>,
}

fn numbers(public: PublicData, subs: Option<SubsData>) -> Numbers {
    let user = public.user;
    let settings = user.as_ref().and_then(|u| u.broadcast_settings.as_ref());
    let stream = user.as_ref().and_then(|u| u.stream.as_ref());
    Numbers {
        live: stream.is_some(),
        title: settings.and_then(|s| s.title.clone()).unwrap_or_default(),
        game: settings
            .and_then(|s| s.game.as_ref())
            .map(|g| g.display_name.clone())
            .unwrap_or_default(),
        viewers: stream.and_then(|s| s.viewers_count).unwrap_or_default(),
        started_at: stream.and_then(|s| s.created_at.clone()).unwrap_or_default(),
        followers: user
            .as_ref()
            .and_then(|u| u.followers.as_ref())
            .and_then(|f| f.total_count),
        subs: subs
            .and_then(|s| s.user)
            .and_then(|u| u.subscriber_score)
            .map(|s| (s.count.unwrap_or_default(), s.current.unwrap_or_default())),
    }
}

/// Both reads in one request; a failed sub read leaves `subs` empty.
pub async fn numbers_now(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Numbers> {
    let vars = json!({ "id": channel_id });
    let mut answers = batch(http, auth, &[(PUBLIC, vars.clone()), (SUBS, vars)])
        .await?
        .into_iter();
    let public: Value = answers.next().unwrap_or(Ok(Value::Null))?;
    let public: PublicData = serde_json::from_value(public).map_err(Error::Decode)?;
    let subs = answers
        .next()
        .and_then(|r| r.ok())
        .and_then(|v| serde_json::from_value::<SubsData>(v).ok());
    Ok(numbers(public, subs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_live_and_offline() {
        let public: PublicData = serde_json::from_str(
            r#"{"user":{"broadcastSettings":{"title":"t","game":{"displayName":"Just Chatting"}},
              "stream":{"createdAt":"2026-10-11T10:00:00Z","viewersCount":42},"followers":{"totalCount":1500}}}"#,
        )
        .unwrap();
        let subs: SubsData =
            serde_json::from_str(r#"{"user":{"subscriberScore":{"count":12,"current":15}}}"#).unwrap();
        let n = numbers(public, Some(subs));
        assert!(n.live);
        assert_eq!((n.viewers, n.followers, n.subs), (42, Some(1500), Some((12, 15))));
        assert_eq!(n.game, "Just Chatting");
        let offline: PublicData =
            serde_json::from_str(r#"{"user":{"broadcastSettings":{"title":"t","game":null},"stream":null}}"#)
                .unwrap();
        let n = numbers(offline, None);
        assert!(!n.live && n.started_at.is_empty() && n.subs.is_none());
    }
}
