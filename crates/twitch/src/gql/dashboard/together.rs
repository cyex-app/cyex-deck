//! 1.35 Stream Together (Twitch's Guest Star) hosted from your own channel: the session,
//! its invites, and the host's moves. Joining someone else's needs a camera and mic on a
//! WebRTC call, which stays on Twitch. D6 Other creators asking to join, and whether
//! guests show on stream as soon as they're set up.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::channel::write;
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

const REQUESTS: &str = "query($id: ID!) { guestStarJoinRequests(hostID: $id) { status inviteMessage \
    user { id login displayName profileImageURL(width: 70) } } }";
const ANSWER: &str = "mutation($input: UpdateGuestStarJoinRequestStatusInput!) { \
    updateGuestStarJoinRequestStatus(input: $input) { error { code } } }";
const AUTO_READ: &str = "query($id: ID!, $login: String!) { \
    guestStarSession(options: { channelID: $id }) { id settings { isAutoPushEnabled } } \
    guestStarSettings(userLogin: $login) { hostSettings { isAutoPushEnabled } } }";
/// No `error` here: top-level errors.
const AUTO_SESSION: &str = "mutation($input: SetGuestStarSessionSettingsInput!) { \
    setGuestStarSessionSettings(input: $input) { session { id } } }";
const AUTO_HOST: &str = "mutation($input: SetGuestStarUserSettingsInput!) { \
    setGuestStarUserSettings(input: $input) { error { code } } }";

/// D6 A creator asking to join your Stream Together.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmJoinRequest")]
pub struct JoinRequest {
    pub user_id: String,
    pub login: String,
    pub name: String,
    pub avatar: String,
    pub message: String,
}

/// D6 Requests still waiting for an answer.
pub async fn join_requests(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Vec<JoinRequest>> {
    let data: Value = query(http, auth, REQUESTS, json!({ "id": channel_id })).await?;
    Ok(requests_from(&data))
}

fn requests_from(data: &Value) -> Vec<JoinRequest> {
    let s = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    data["guestStarJoinRequests"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter(|r| r["status"].as_str().is_none_or(|s| s == "PENDING"))
                .filter_map(|r| {
                    let u = &r["user"];
                    let login = u["login"].as_str()?.to_owned();
                    Some(JoinRequest {
                        user_id: u["id"].as_str()?.to_owned(),
                        name: u["displayName"].as_str().map_or_else(|| login.clone(), str::to_owned),
                        login,
                        avatar: s(&u["profileImageURL"]),
                        message: s(&r["inviteMessage"]),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// D6 Lets them in, or not.
pub async fn answer(http: &Http, auth: Auth<'_>, user_id: &str, approve: bool) -> Result<()> {
    let status = if approve { "APPROVED" } else { "DENIED" };
    let input = json!({ "userID": user_id, "requestStatus": status });
    write(http, auth, None, ANSWER, "updateGuestStarJoinRequestStatus", input).await.map(drop)
}

/// D6 Whether guests show on stream once they're set up: the session's setting while one
/// runs, else the host's default for the next.
pub async fn auto_add(http: &Http, auth: Auth<'_>, channel_id: &str, login: &str) -> Result<Option<bool>> {
    let data: Value = query(http, auth, AUTO_READ, json!({ "id": channel_id, "login": login })).await?;
    Ok(auto_from(&data))
}

fn auto_from(data: &Value) -> Option<bool> {
    data["guestStarSession"]["settings"]["isAutoPushEnabled"]
        .as_bool()
        .or_else(|| data["guestStarSettings"]["hostSettings"]["isAutoPushEnabled"].as_bool())
}

/// D6 Sets it for the running session, or as the host's default.
pub async fn set_auto_add(http: &Http, auth: Auth<'_>, session_id: Option<&str>, on: bool) -> Result<()> {
    match session_id {
        Some(id) => {
            let input = json!({ "sessionID": id, "isAutoPushEnabled": on });
            query::<Value>(http, auth, AUTO_SESSION, json!({ "input": input })).await?;
            Ok(())
        }
        None => write(http, auth, None, AUTO_HOST, "setGuestStarUserSettings", json!({ "isAutoPushEnabled": on })).await.map(drop),
    }
}

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
    write(
        http,
        auth,
        None,
        START,
        "startGuestStarSession",
        json!({ "channelID": channel_id }),
    )
    .await
    .map(drop)
}

pub async fn end(http: &Http, auth: Auth<'_>, session_id: &str) -> Result<()> {
    write(
        http,
        auth,
        None,
        END,
        "endGuestStarSession",
        json!({ "sessionID": session_id }),
    )
    .await
    .map(drop)
}

fn guest_input(session_id: &str, guest_id: &str) -> Value {
    json!({ "sessionID": session_id, "guestID": guest_id })
}

pub async fn invite(http: &Http, auth: Auth<'_>, session_id: &str, guest_id: &str) -> Result<()> {
    let input = guest_input(session_id, guest_id);
    write(http, auth, None, INVITE, "inviteUserToGuestStarSession", input).await.map(drop)
}

pub async fn uninvite(http: &Http, auth: Auth<'_>, session_id: &str, guest_id: &str) -> Result<()> {
    let input = guest_input(session_id, guest_id);
    write(http, auth, None, UNINVITE, "removeGuestStarInvite", input).await.map(drop)
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
    write(http, auth, None, ASSIGN, "assignUserToGuestStarSlot", input).await.map(drop)
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
    write(http, auth, None, UNASSIGN, "removeUserFromGuestStarSlot", input).await.map(drop)
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

    #[test]
    fn join_requests_and_auto_add_read() {
        let data = json!({ "guestStarJoinRequests": [
            { "status": "PENDING", "inviteMessage": "collab?", "user": { "id": "9", "login": "kuni", "displayName": "Kuni",
                "profileImageURL": "https://p" } },
            { "status": "DENIED", "user": { "id": "8", "login": "x" } } ] });
        let r = requests_from(&data);
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].name.as_str(), r[0].message.as_str()), ("Kuni", "collab?"));
        let session = json!({ "guestStarSession": { "settings": { "isAutoPushEnabled": false } },
            "guestStarSettings": { "hostSettings": { "isAutoPushEnabled": true } } });
        assert_eq!(auto_from(&session), Some(false));
        let host = json!({ "guestStarSession": null, "guestStarSettings": { "hostSettings": { "isAutoPushEnabled": true } } });
        assert_eq!(auto_from(&host), Some(true));
        assert_eq!(auto_from(&json!({})), None);
    }
}
