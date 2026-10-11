//! 1.35 Stream Together (Twitch's Guest Star) hosted from your own channel: the session,
//! its invites, and the host's moves. Joining someone else's needs a camera and mic on a
//! WebRTC call, which stays on Twitch.

use serde::Deserialize;
use serde_json::{Value, json};

use super::mutate;
use crate::gql::{Auth, query};
use crate::helix::gueststar::{Guest, Session};
use crate::{Http, Result};

const READ: &str = "query($id: ID!) { guestStarSession(options: { channelID: $id }) { id \
    guests { id isSlotLive user { id login displayName } } \
    invites(first: 25) { edges { node { readyState guest { id login displayName } } } } } }";
const START: &str = "mutation($input: StartGuestStarSessionInput!) { \
    startGuestStarSession(input: $input) { error { code } } }";
const END: &str =
    "mutation($input: EndGuestStarSessionInput!) { endGuestStarSession(input: $input) { error { code } } }";
const INVITE: &str = "mutation($input: InviteUserToGuestStarSessionInput!) { \
    inviteUserToGuestStarSession(input: $input) { error { code } } }";
const UNINVITE: &str = "mutation($input: RemoveGuestStarInviteInput!) { \
    removeGuestStarInvite(input: $input) { error { code } } }";
const ASSIGN: &str = "mutation($input: AssignUserToGuestStarSlotInput!) { \
    assignUserToGuestStarSlot(input: $input) { error { code } } }";
const UNASSIGN: &str = "mutation($input: RemoveUserFromGuestStarSlotInput!) { \
    removeUserFromGuestStarSlot(input: $input) { error { code } } }";

/// A pending invite, with the guest's name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Invited {
    pub user_id: String,
    pub login: String,
    pub name: String,
    /// Twitch's ready state, e.g. `INVITED` or `READY`.
    pub status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    guest_star_session: Option<Raw>,
}

#[derive(Deserialize)]
struct Raw {
    id: String,
    #[serde(default)]
    guests: Vec<RawGuest>,
    invites: Option<Edges>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGuest {
    /// The slot.
    id: String,
    #[serde(default)]
    is_slot_live: bool,
    user: Option<User>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: String,
    login: String,
    display_name: String,
}

#[derive(Deserialize)]
struct Edges {
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    node: Node,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    ready_state: Option<String>,
    guest: Option<User>,
}

fn from_raw(raw: Raw) -> (Session, Vec<Invited>) {
    let guests = raw
        .guests
        .into_iter()
        .filter_map(|g| {
            let u = g.user?;
            Some(Guest {
                slot_id: g.id,
                user_id: u.id,
                user_login: u.login,
                user_name: u.display_name,
                is_live: g.is_slot_live,
            })
        })
        .collect();
    let invites = raw
        .invites
        .map(|e| e.edges)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let u = e.node.guest?;
            Some(Invited {
                user_id: u.id,
                login: u.login,
                name: u.display_name,
                status: e.node.ready_state.unwrap_or_default(),
            })
        })
        .collect();
    (Session { id: raw.id, guests }, invites)
}

/// The running session and its invites; `None` when there's no session.
pub async fn session(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
) -> Result<Option<(Session, Vec<Invited>)>> {
    let d: Data = query(http, auth, READ, json!({ "id": channel_id })).await?;
    Ok(d.guest_star_session.map(from_raw))
}

pub async fn start(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<()> {
    mutate(
        http,
        auth,
        START,
        "startGuestStarSession",
        json!({ "channelID": channel_id }),
    )
    .await
}

pub async fn end(http: &Http, auth: Auth<'_>, session_id: &str) -> Result<()> {
    mutate(
        http,
        auth,
        END,
        "endGuestStarSession",
        json!({ "sessionID": session_id }),
    )
    .await
}

fn guest_input(session_id: &str, guest_id: &str) -> Value {
    json!({ "sessionID": session_id, "guestID": guest_id })
}

pub async fn invite(http: &Http, auth: Auth<'_>, session_id: &str, guest_id: &str) -> Result<()> {
    let input = guest_input(session_id, guest_id);
    mutate(http, auth, INVITE, "inviteUserToGuestStarSession", input).await
}

pub async fn uninvite(http: &Http, auth: Auth<'_>, session_id: &str, guest_id: &str) -> Result<()> {
    let input = guest_input(session_id, guest_id);
    mutate(http, auth, UNINVITE, "removeGuestStarInvite", input).await
}

pub async fn assign(
    http: &Http,
    auth: Auth<'_>,
    session_id: &str,
    guest_id: &str,
    slot_id: &str,
) -> Result<()> {
    let mut input = guest_input(session_id, guest_id);
    input["slotID"] = slot_id.into();
    mutate(http, auth, ASSIGN, "assignUserToGuestStarSlot", input).await
}

/// Takes a guest out of their slot; they stay invited.
pub async fn unassign(
    http: &Http,
    auth: Auth<'_>,
    session_id: &str,
    guest_id: &str,
    slot_id: &str,
) -> Result<()> {
    let mut input = guest_input(session_id, guest_id);
    input["slotID"] = slot_id.into();
    input["shouldReInviteGuest"] = true.into();
    mutate(http, auth, UNASSIGN, "removeUserFromGuestStarSlot", input).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_maps_guests_and_invites() {
        let d: Data = serde_json::from_str(
            r#"{"guestStarSession":{"id":"s1","guests":[
                {"id":"0","isSlotLive":true,"user":{"id":"7","login":"me","displayName":"Me"}},
                {"id":"1","isSlotLive":false,"user":null}],
              "invites":{"edges":[{"node":{"readyState":"READY","guest":{"id":"9","login":"kuni","displayName":"Kuni"}}}]}}}"#,
        )
        .unwrap();
        let (s, inv) = from_raw(d.guest_star_session.unwrap());
        assert_eq!(s.id, "s1");
        assert_eq!(s.guests.len(), 1);
        assert_eq!((s.guests[0].slot_id.as_str(), s.guests[0].is_live), ("0", true));
        assert_eq!((inv[0].name.as_str(), inv[0].status.as_str()), ("Kuni", "READY"));
    }
}
