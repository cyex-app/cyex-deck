//! Playback tokens (1.1) for live streams and VODs. They sign usher's playlists. Signed
//! out they need no integrity token; live ones last 20 minutes, VOD ones 20 hours.
//! Twitch's site sends this raw query too. Checked 2026/10/09.

use serde::Deserialize;
use serde_json::json;

use super::{Auth, query_with};
use crate::{Error, Http, Result};

const TOKEN: &str = "query PlaybackAccessToken_Template($login: String!, $isLive: Boolean!, $vodID: ID!, \
    $isVod: Boolean!, $playerType: String!, $platform: String!) { \
    streamPlaybackAccessToken(channelName: $login, params: {platform: $platform, playerBackend: \"mediaplayer\", \
    playerType: $playerType}) @include(if: $isLive) { value signature authorization { isForbidden forbiddenReasonCode } } \
    videoPlaybackAccessToken(id: $vodID, params: {platform: $platform, playerBackend: \"mediaplayer\", \
    playerType: $playerType}) @include(if: $isVod) { value signature } }";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AccessToken {
    /// JSON, passed to usher as is.
    pub value: String,
    pub signature: String,
}

/// Who's asking for the stream: Twitch hands out different ads per player type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Player<'a> {
    pub player_type: &'a str,
    pub platform: &'a str,
}

impl Player<'static> {
    /// What twitch.tv's own player asks as.
    pub const SITE: Player<'static> = Player {
        player_type: "site",
        platform: "web",
    };
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    stream_playback_access_token: Option<RawToken>,
    video_playback_access_token: Option<RawToken>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawToken {
    value: String,
    signature: String,
    authorization: Option<RawAuthorization>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAuthorization {
    is_forbidden: bool,
    forbidden_reason_code: Option<String>,
}

/// A live stream's token. Twitch issues one for offline channels too; usher then says
/// the channel can't be found. `device_id` is sent as `X-Device-Id`, like twitch.tv does.
pub async fn stream_token(
    http: &Http,
    auth: Auth<'_>,
    device_id: Option<&str>,
    login: &str,
    player: Player<'_>,
) -> Result<AccessToken> {
    let vars = json!({
        "isLive": true, "login": login, "isVod": false, "vodID": "",
        "playerType": player.player_type, "platform": player.platform,
    });
    let data: Data = query_with(http, auth, &device(device_id), TOKEN, vars).await?;
    token(data.stream_playback_access_token)
}

pub async fn video_token(http: &Http, auth: Auth<'_>, device_id: Option<&str>, id: &str) -> Result<AccessToken> {
    let vars = json!({
        "isLive": false, "login": "", "isVod": true, "vodID": id,
        "playerType": Player::SITE.player_type, "platform": Player::SITE.platform,
    });
    let data: Data = query_with(http, auth, &device(device_id), TOKEN, vars).await?;
    token(data.video_playback_access_token)
}

fn device(id: Option<&str>) -> Vec<(&'static str, &str)> {
    id.map(|id| ("X-Device-Id", id)).into_iter().collect()
}

fn token(raw: Option<RawToken>) -> Result<AccessToken> {
    let raw = raw.ok_or(Error::Api {
        status: 404,
        message: "no playback token".into(),
    })?;
    if let Some(a) = raw.authorization.filter(|a| a.is_forbidden) {
        return Err(Error::Api {
            status: 403,
            message: a.forbidden_reason_code.unwrap_or_else(|| "forbidden".into()),
        });
    }
    Ok(AccessToken {
        value: raw.value,
        signature: raw.signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_and_refusals() {
        let ok: Data = serde_json::from_str(
            r#"{"streamPlaybackAccessToken":{"value":"{\"channel\":\"x\"}","signature":"ab12",
              "authorization":{"isForbidden":false,"forbiddenReasonCode":"NONE"},"__typename":"PlaybackAccessToken"}}"#,
        )
        .unwrap();
        let t = token(ok.stream_playback_access_token).unwrap();
        assert_eq!((t.value.as_str(), t.signature.as_str()), ("{\"channel\":\"x\"}", "ab12"));

        let geo: Data = serde_json::from_str(
            r#"{"streamPlaybackAccessToken":{"value":"{}","signature":"x",
              "authorization":{"isForbidden":true,"forbiddenReasonCode":"GEOBLOCKED"}}}"#,
        )
        .unwrap();
        let e = token(geo.stream_playback_access_token).unwrap_err();
        assert_eq!(e.api_message(), Some("GEOBLOCKED"));

        let vod: Data = serde_json::from_str(r#"{"videoPlaybackAccessToken":null}"#).unwrap();
        assert!(token(vod.video_playback_access_token).is_err());
    }
}
