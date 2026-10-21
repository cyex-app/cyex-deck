//! V107-R36 Who holds which role in the broadcaster's own channel, and how many VIP slots it
//! has, as the dashboard's Roles Manager reads them (`UserRolesCacheQuery`,
//! `VIPArtistStatusQuery`; its roles-page chunk, 2026/10/21). Only the broadcaster may ask.
//! V107-R41 Reads through the dashboard's `Channel`; a list with more pages goes on alone,
//! paced, until a cursor repeats. V107-R42 Lives under `dashboard::roles` now.

use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use crate::gql::connection::{GAP, Page, UserRef, page as connection_page, paginate};
use crate::gql::dashboard::Channel;
use crate::Result;

/// Someone holding a role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleHolder {
    pub id: String,
    pub login: String,
    pub display: String,
    pub avatar: String,
    /// When the role was given (RFC 3339), if Twitch says.
    #[ts(optional = nullable)]
    pub granted_at: Option<String>,
}

/// Every role list of a channel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoleLists {
    pub lead_mods: Vec<RoleHolder>,
    pub mods: Vec<RoleHolder>,
    pub vips: Vec<RoleHolder>,
    pub editors: Vec<RoleHolder>,
    pub artists: Vec<RoleHolder>,
    pub agents: Vec<RoleHolder>,
}

const NODE: &str = "edges{ cursor grantedAt node{ id login displayName profileImageURL(width:50) } } pageInfo{ hasNextPage }";

fn list_query() -> String {
    format!(
        "query UserRolesCacheQuery($channelID: ID!, $includeEditors: Boolean!, $includeMods: Boolean!, $includeVIPs: Boolean!, \
         $includeArtists: Boolean!, $includeLeadMods: Boolean!, $includeAgents: Boolean!, $modsCursor: Cursor, $vipsCursor: Cursor, \
         $artistsCursor: Cursor, $leadModsCursor: Cursor, $agentsCursor: Cursor) {{ \
         artists: usersByCommunityRole(channelID: $channelID, role: ARTIST, first: 100, after: $artistsCursor) @include(if: $includeArtists) {{ {NODE} }} \
         user(id: $channelID) {{ id \
           editors @include(if: $includeEditors) {{ {NODE} }} \
           mods(first: 100, after: $modsCursor) @include(if: $includeMods) {{ {NODE} }} \
           vips(first: 100, after: $vipsCursor) @include(if: $includeVIPs) {{ {NODE} }} }} \
         channel(id: $channelID) {{ id \
           leadMods: usersWithRole(id: \"lead_mod\", first: 25, after: $leadModsCursor) @include(if: $includeLeadMods) {{ {NODE} }} \
           agents: usersWithRole(id: \"pre_agent\", first: 25, after: $agentsCursor) @include(if: $includeAgents) {{ {NODE} }} }} }}"
    )
}

/// The lists, in the order the page cursors are kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Artists,
    Editors,
    Mods,
    Vips,
    LeadMods,
    Agents,
}

const PARTS: [Part; 6] = [Part::Artists, Part::Editors, Part::Mods, Part::Vips, Part::LeadMods, Part::Agents];

impl Part {
    fn pointer(self) -> &'static str {
        match self {
            Part::Artists => "/artists",
            Part::Editors => "/user/editors",
            Part::Mods => "/user/mods",
            Part::Vips => "/user/vips",
            Part::LeadMods => "/channel/leadMods",
            Part::Agents => "/channel/agents",
        }
    }

    /// The variable that turns it on, and its cursor's (editors have none).
    fn vars(self) -> (&'static str, Option<&'static str>) {
        match self {
            Part::Artists => ("includeArtists", Some("artistsCursor")),
            Part::Editors => ("includeEditors", None),
            Part::Mods => ("includeMods", Some("modsCursor")),
            Part::Vips => ("includeVIPs", Some("vipsCursor")),
            Part::LeadMods => ("includeLeadMods", Some("leadModsCursor")),
            Part::Agents => ("includeAgents", Some("agentsCursor")),
        }
    }

    fn of(self, l: &mut RoleLists) -> &mut Vec<RoleHolder> {
        match self {
            Part::Artists => &mut l.artists,
            Part::Editors => &mut l.editors,
            Part::Mods => &mut l.mods,
            Part::Vips => &mut l.vips,
            Part::LeadMods => &mut l.lead_mods,
            Part::Agents => &mut l.agents,
        }
    }
}

/// One page of one list: who, and the cursor for the next page when there is one.
fn page(data: &Value, part: Part) -> (Vec<RoleHolder>, Option<String>) {
    let p = connection_page(data.pointer(part.pointer()).unwrap_or(&Value::Null), holder);
    (p.items, p.next)
}

fn holder(edge: &Value) -> Option<RoleHolder> {
    let UserRef { id, login, display, avatar } = UserRef::from_node(edge.get("node")?)?;
    Some(RoleHolder {
        id,
        login,
        display,
        avatar,
        granted_at: edge.get("grantedAt").and_then(Value::as_str).filter(|g| !g.is_empty()).map(str::to_owned),
    })
}

fn vars(channel_id: &str, want: &[(Part, Option<String>)]) -> Value {
    let mut v = json!({ "channelID": channel_id });
    for p in PARTS {
        let (on, cursor) = p.vars();
        let asked = want.iter().find(|(w, _)| *w == p);
        v[on] = json!(asked.is_some());
        if let Some(c) = cursor {
            v[c] = json!(asked.and_then(|(_, c)| c.clone()));
        }
    }
    v
}

/// At most this many pages per list after the first (100 per page for mods and VIPs).
const MAX_PAGES: usize = 20;

/// Every role holder of the channel: one read for every list's first page, then each list
/// with more on its own. The flag says a list was cut short.
pub async fn role_lists(ch: &Channel<'_>) -> Result<(RoleLists, bool)> {
    let q = list_query();
    let keep = |v: &Value| v.get("user").is_some_and(|u| !u.is_null());
    let all: Vec<(Part, Option<String>)> = PARTS.iter().map(|p| (*p, None)).collect();
    let data = ch.read_partial(&q, vars(ch.id, &all), keep).await?;
    let mut out = RoleLists::default();
    let mut truncated = false;
    for p in PARTS {
        let (people, next) = page(&data, p);
        p.of(&mut out).extend(people);
        if next.is_none() {
            continue;
        }
        let q = q.as_str();
        let more = paginate(next, MAX_PAGES, GAP, |cursor| async move {
            let data = ch.read_partial(q, vars(ch.id, &[(p, cursor)]), keep).await?;
            let (items, next) = page(&data, p);
            Ok(Page { items, next })
        })
        .await;
        match more {
            Ok(m) => {
                p.of(&mut out).extend(m.items);
                truncated |= m.truncated;
            }
            Err(_) => truncated = true,
        }
    }
    Ok((out, truncated))
}

/// How many VIPs and artists the channel may have. `None`: Twitch didn't say.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slots {
    pub vips: Option<u32>,
    pub artists: Option<u32>,
}

const SLOTS: &str = "query VIPArtistStatusQuery($channelID: ID!, $isArtistAlleyFeatureEnabled: Boolean!) { channel(id: $channelID) { id numberOfVIPsAllowed } channelCommunityRolesSettings(channelID: $channelID) @include(if: $isArtistAlleyFeatureEnabled) { roleSettings { limit role } } }";

fn slots_of(data: &Value) -> Slots {
    let n = |v: Option<&Value>| v.and_then(Value::as_u64).map(|n| n.min(u32::MAX as u64) as u32);
    let artists = data
        .pointer("/channelCommunityRolesSettings/roleSettings")
        .and_then(Value::as_array)
        .and_then(|a| a.iter().find(|r| r.get("role").and_then(Value::as_str) == Some("ARTIST")))
        .and_then(|r| n(r.get("limit")));
    Slots { vips: n(data.pointer("/channel/numberOfVIPsAllowed")), artists }
}

/// The channel's VIP and artist slots.
pub async fn slots(ch: &Channel<'_>) -> Result<Slots> {
    let data: Value = ch.read(SLOTS, json!({ "channelID": ch.id, "isArtistAlleyFeatureEnabled": true })).await?;
    Ok(slots_of(&data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(login: &str, at: &str) -> Value {
        json!({ "cursor": format!("c-{login}"), "grantedAt": at, "node": { "id": format!("id-{login}"), "login": login, "displayName": login.to_uppercase(), "profileImageURL": "https://x/a.png" } })
    }

    #[test]
    fn v107_r36_reads_every_list_and_its_cursor() {
        let d = json!({
            "artists": { "edges": [edge("art", "2024-01-02T03:04:05Z")], "pageInfo": { "hasNextPage": false } },
            "user": {
                "editors": { "edges": [edge("ed", "")], "pageInfo": { "hasNextPage": false } },
                "mods": { "edges": [edge("m1", "2020-01-01T00:00:00Z"), edge("m2", "2021-01-01T00:00:00Z")], "pageInfo": { "hasNextPage": true } },
                "vips": null
            },
            "channel": { "leadMods": { "edges": [{ "node": null }], "pageInfo": { "hasNextPage": false } } }
        });
        let (mods, next) = page(&d, Part::Mods);
        assert_eq!(mods.len(), 2);
        assert_eq!(next.as_deref(), Some("c-m2"));
        assert_eq!(mods[0].display, "M1");
        assert_eq!(mods[0].granted_at.as_deref(), Some("2020-01-01T00:00:00Z"));
        let (ed, next) = page(&d, Part::Editors);
        assert_eq!((ed[0].login.as_str(), ed[0].granted_at.clone(), next), ("ed", None, None));
        assert!(page(&d, Part::Vips).0.is_empty());
        assert!(page(&d, Part::LeadMods).0.is_empty(), "a null node is skipped");
        assert!(page(&d, Part::Agents).0.is_empty());
        assert_eq!(page(&d, Part::Artists).0[0].id, "id-art");
    }

    #[test]
    fn v107_r36_asks_only_for_lists_with_more() {
        let v = vars("1", &[(Part::Mods, Some("c".into()))]);
        assert_eq!(v["includeMods"], json!(true));
        assert_eq!(v["modsCursor"], json!("c"));
        assert_eq!(v["includeVIPs"], json!(false));
        assert_eq!(v["vipsCursor"], Value::Null);
        assert_eq!(v["includeEditors"], json!(false));
        let all = vars("1", &PARTS.map(|p| (p, None)));
        assert!(PARTS.iter().all(|p| all[p.vars().0] == json!(true)));
        assert!(list_query().contains("usersWithRole(id: \"pre_agent\""));
    }

    #[test]
    fn v107_r36_reads_slots() {
        let d = json!({ "channel": { "numberOfVIPsAllowed": 10 }, "channelCommunityRolesSettings": { "roleSettings": [{ "role": "ARTIST", "limit": 5 }] } });
        assert_eq!(slots_of(&d), Slots { vips: Some(10), artists: Some(5) });
        let d = json!({ "channel": { "numberOfVIPsAllowed": 0 } });
        assert_eq!(slots_of(&d), Slots { vips: Some(0), artists: None });
        assert_eq!(slots_of(&json!({ "channel": { "numberOfVIPsAllowed": null } })), Slots::default());
    }

    use crate::gql::dashboard::mock::{ID, serve};

    fn part(path: &[&str], logins: &[&str], cursor: &str, more: bool) -> Value {
        let edges: Vec<Value> = logins.iter().map(|l| json!({ "cursor": cursor, "node": { "id": format!("id-{l}"), "login": l } })).collect();
        let mut v = json!({ "edges": edges, "pageInfo": { "hasNextPage": more } });
        for k in path.iter().rev() {
            v = json!({ *k: v });
        }
        v
    }

    #[tokio::test]
    async fn v107_r41_a_list_with_more_goes_on_alone_until_a_cursor_repeats() {
        let mut first = part(&["user", "mods"], &["m1"], "c1", true);
        first["user"]["vips"] = part(&["x"], &["v1"], "", false)["x"].clone();
        first["channel"] = json!({ "leadMods": { "edges": [], "pageInfo": { "hasNextPage": false } } });
        let m = serve([
            json!({ "data": first }).to_string(),
            json!({ "data": part(&["user", "mods"], &["m2"], "c2", true) }).to_string(),
            json!({ "data": part(&["user", "mods"], &["m3"], "c1", true) }).to_string(),
        ]);
        let (lists, truncated) = role_lists(&m.channel()).await.unwrap();
        let mods: Vec<&str> = lists.mods.iter().map(|h| h.login.as_str()).collect();
        assert_eq!(mods, ["m1", "m2", "m3"]);
        assert_eq!(lists.vips.len(), 1);
        assert!(truncated, "c1 came back");
        let sent = m.sent();
        assert_eq!(sent[0]["variables"]["channelID"], ID);
        assert_eq!(sent[0]["variables"]["includeVIPs"], true);
        assert_eq!(sent[1]["variables"]["includeVIPs"], false);
        assert_eq!(sent[1]["variables"]["modsCursor"], "c1");
        assert_eq!(sent[2]["variables"]["modsCursor"], "c2");
    }
}
