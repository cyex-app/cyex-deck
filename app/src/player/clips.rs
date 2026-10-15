//! P9, V102-08 Making a clip from the player as the tile's account: Helix when it was
//! granted `clips:edit`, else its GQL sign-in, the way Twitch's player does it. Anonymous
//! tabs can't clip, and neither can anyone while the channel is offline.
//! V103-09 With Sign in with Twitch, Twitch's own clip editor opens right away in the
//! account's Twitch window (start, end, title, publish). Without it the clip is the last
//! 30 seconds, made as is.

use cyex_twitch::gql::clip_create;
use cyex_twitch::helix;
use cyex_twitch::{Error, gql};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use ts_rs::TS;

use crate::auth::{self, Auth};

const OFFLINE: &str = "Clips only work while the channel is live.";

/// A clip just made (V103-09).
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MadeClip {
    pub slug: String,
    /// Twitch's editor opened in the account's Twitch window. False: the default last
    /// 30 seconds, nothing to edit here.
    pub editing: bool,
}

pub async fn create(app: &AppHandle, account: &str, channel_id: &str) -> Result<MadeClip, String> {
    let http = app.state::<Auth>().http.clone();
    if let Some((token, client_id)) = auth::helix_scoped(app, account, "clips:edit") {
        let auth = helix::Auth {
            client_id: &client_id,
            token: &token,
        };
        let made = helix::create_clip(&http, auth, channel_id)
            .await
            .map_err(helix_error)?;
        // Twitch keeps the editor open until the clip is published, up to 24 hours.
        let editing = match crate::windows::twitch::open(app, account, &made.edit_url).await {
            Ok(()) => true,
            Err(e) => {
                eprintln!("clips: couldn't open the editor: {e}");
                false
            }
        };
        return Ok(MadeClip {
            slug: made.id,
            editing,
        });
    }
    let (token, client_id) = auth::gql_token(app, account).ok_or("Clips need this tab to be signed in.")?;
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let (broadcast, started) = clip_create::live_broadcast(&http, auth, channel_id)
        .await
        .map_err(gql_error)?
        .ok_or(OFFLINE)?;
    let offset = live_edge(&started, OffsetDateTime::now_utc()).ok_or(OFFLINE)?;
    let slug = clip_create::create_clip(&http, auth, channel_id, &broadcast, offset)
        .await
        .map_err(gql_error)?;
    Ok(MadeClip { slug, editing: false })
}

/// Seconds into the broadcast right now: the clip ends there.
fn live_edge(started_at: &str, now: OffsetDateTime) -> Option<f64> {
    let started = OffsetDateTime::parse(started_at, &Rfc3339).ok()?;
    let secs = (now - started).as_seconds_f64();
    (secs > 0.0).then_some(secs)
}

fn helix_error(e: Error) -> String {
    let m = e.api_message().unwrap_or_default().to_ascii_lowercase();
    match e.status() {
        None if e.is_network() => "Couldn’t reach Twitch.".into(),
        Some(404) => OFFLINE.into(),
        _ if m.contains("live") || m.contains("offline") => OFFLINE.into(),
        Some(401) => "Sign in again in Settings → Accounts to allow clips.".into(),
        Some(403) => "This channel doesn’t allow clips right now.".into(),
        _ => e.to_string(),
    }
}

fn gql_error(e: Error) -> String {
    match &e {
        Error::Network(_) => "Couldn’t reach Twitch.".into(),
        Error::Refused(code) => refused(code).into(),
        _ => e.to_string(),
    }
}

/// Twitch's `CreateClipErrorCode`s, worded short.
fn refused(code: &str) -> &'static str {
    match code {
        "CHANNEL_NOT_LIVE" => OFFLINE,
        "CHANNEL_NOT_CLIPPABLE" | "CATEGORY_NOT_CLIPPABLE" => "This channel can’t be clipped right now.",
        "USER_BANNED" | "USER_TIMED_OUT" | "USER_RESTRICTED" | "CHANNEL_BANNED" => {
            "You can’t clip in this channel right now."
        }
        "REQUEST_THROTTLED" => "Too many clips at once. Try again in a moment.",
        _ => "Twitch didn’t make the clip. Try again.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clip_ends_at_the_live_edge() {
        let now = OffsetDateTime::parse("2026-10-11T10:00:30Z", &Rfc3339).unwrap();
        assert_eq!(live_edge("2026-10-11T10:00:00Z", now), Some(30.0));
        assert_eq!(live_edge("2026-10-11T11:00:00Z", now), None);
        assert_eq!(live_edge("not a date", now), None);
    }

    #[test]
    fn twitchs_codes_read_short() {
        assert_eq!(refused("CHANNEL_NOT_LIVE"), OFFLINE);
        assert!(refused("REQUEST_THROTTLED").starts_with("Too many"));
        assert!(refused("SOMETHING_NEW").starts_with("Twitch didn’t"));
        let offline = Error::Api {
            status: 404,
            message: "The broadcaster must be broadcasting live.".into(),
        };
        assert_eq!(helix_error(offline), OFFLINE);
    }
}
