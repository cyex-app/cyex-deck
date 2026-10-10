//! 1.35 Stream Together, Twitch's co-stream feature, over the beta Guest Star Helix API
//! (`channel:manage:guest_star`). Core only (D26): read/create/end a session, invite and
//! cancel guests, assign and clear slots. Joining someone else's session happens on Twitch.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Auth, Data, get, post, request};
use crate::http::{empty, json};
use crate::{Http, Result};

/// Someone in a Stream Together session and the slot they hold.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export, rename = "TogetherGuest")]
pub struct Guest {
    /// The slot in the layout, `"0"` for the host. Empty until assigned.
    #[serde(default)]
    pub slot_id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub user_login: String,
    #[serde(rename(deserialize = "user_display_name"), default)]
    pub user_name: String,
    #[serde(default)]
    pub is_live: bool,
}

/// A running Stream Together session.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export, rename = "TogetherSession")]
pub struct Session {
    pub id: String,
    #[serde(default)]
    pub guests: Vec<Guest>,
}

/// A pending invite.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export, rename = "TogetherInvite")]
pub struct Invite {
    pub user_id: String,
    /// RFC 3339.
    #[serde(default)]
    pub invited_at: String,
    /// `INVITED`, `ACCEPTED` or `READY`.
    #[serde(default)]
    pub status: String,
}

/// The ids every call inside a session names: the channel, who acts (the broadcaster
/// acting as their own mod), and the session.
#[derive(Clone, Copy)]
pub struct InSession<'a> {
    pub broadcaster_id: &'a str,
    pub moderator_id: &'a str,
    pub session_id: &'a str,
}

impl InSession<'_> {
    fn query(&self) -> [(&'static str, &str); 3] {
        [
            ("broadcaster_id", self.broadcaster_id),
            ("moderator_id", self.moderator_id),
            ("session_id", self.session_id),
        ]
    }
}

/// The session now, or `None` when the channel isn't in Stream Together.
pub async fn session(
    http: &Http,
    auth: Auth<'_>,
    broadcaster_id: &str,
    moderator_id: &str,
) -> Result<Option<Session>> {
    let req = get(http, auth, "/guest_star/session")
        .query(&[("broadcaster_id", broadcaster_id), ("moderator_id", moderator_id)]);
    let d: Data<Session> = json(req).await?;
    Ok(d.data.into_iter().next().filter(|s| !s.id.is_empty()))
}

/// Starts a session; the host takes the first slot.
pub async fn create_session(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<Session> {
    let req = post(http, auth, "/guest_star/session").query(&[("broadcaster_id", broadcaster_id)]);
    let d: Data<Session> = json(req).await?;
    Ok(d.data.into_iter().next().unwrap_or_default())
}

pub async fn end_session(http: &Http, auth: Auth<'_>, broadcaster_id: &str, session_id: &str) -> Result<()> {
    let req = request(http, auth, Method::DELETE, "/guest_star/session")
        .query(&[("broadcaster_id", broadcaster_id), ("session_id", session_id)]);
    empty(req).await
}

pub async fn invites(http: &Http, auth: Auth<'_>, at: InSession<'_>) -> Result<Vec<Invite>> {
    let req = get(http, auth, "/guest_star/invites").query(&at.query());
    let d: Data<Invite> = json(req).await?;
    Ok(d.data)
}

pub async fn send_invite(http: &Http, auth: Auth<'_>, at: InSession<'_>, guest_id: &str) -> Result<()> {
    let req = post(http, auth, "/guest_star/invites")
        .query(&at.query())
        .query(&[("guest_id", guest_id)]);
    empty(req).await
}

pub async fn delete_invite(http: &Http, auth: Auth<'_>, at: InSession<'_>, guest_id: &str) -> Result<()> {
    let req = request(http, auth, Method::DELETE, "/guest_star/invites")
        .query(&at.query())
        .query(&[("guest_id", guest_id)]);
    empty(req).await
}

/// Puts a guest into a slot in the layout.
pub async fn assign_slot(
    http: &Http,
    auth: Auth<'_>,
    at: InSession<'_>,
    guest_id: &str,
    slot_id: &str,
) -> Result<()> {
    let req = post(http, auth, "/guest_star/slot")
        .query(&at.query())
        .query(&[("guest_id", guest_id), ("slot_id", slot_id)]);
    empty(req).await
}

/// Removes a guest from their slot. `reinvite` keeps them invited to the session.
pub async fn delete_slot(
    http: &Http,
    auth: Auth<'_>,
    at: InSession<'_>,
    guest_id: &str,
    slot_id: &str,
    reinvite: bool,
) -> Result<()> {
    let req = request(http, auth, Method::DELETE, "/guest_star/slot")
        .query(&at.query())
        .query(&[
            ("guest_id", guest_id),
            ("slot_id", slot_id),
            ("should_reinvite_guest", if reinvite { "true" } else { "false" }),
        ]);
    empty(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_and_guests_parse() {
        let d: Data<Session> = serde_json::from_str(
            r#"{"data":[{"id":"sess-1","guests":[
              {"slot_id":"0","is_live":true,"user_id":"1","user_login":"host","user_display_name":"Host","volume":100},
              {"slot_id":"1","is_live":true,"user_id":"2","user_login":"guest","user_display_name":"Guest","volume":100}
            ]}]}"#,
        )
        .unwrap();
        let s = d.data.into_iter().next().unwrap();
        assert_eq!(s.id, "sess-1");
        assert_eq!(
            (
                s.guests.len(),
                s.guests[1].user_login.as_str(),
                s.guests[1].slot_id.as_str()
            ),
            (2, "guest", "1")
        );
    }

    #[test]
    fn empty_session_is_none() {
        let d: Data<Session> = serde_json::from_str(r#"{"data":[]}"#).unwrap();
        assert!(d.data.into_iter().next().filter(|s| !s.id.is_empty()).is_none());
    }

    #[test]
    fn invite_parses() {
        let d: Data<Invite> = serde_json::from_str(
            r#"{"data":[{"user_id":"7","invited_at":"2024-01-01T00:00:00Z","status":"INVITED",
            "is_video_enabled":true,"is_audio_enabled":true,"is_video_available":true,"is_audio_available":true}]}"#,
        )
        .unwrap();
        assert_eq!(
            (d.data[0].user_id.as_str(), d.data[0].status.as_str()),
            ("7", "INVITED")
        );
    }
}
