//! V111-13 Where someone is mod, VIP, founder or artist in other channels, from roles.tv
//! (public, read only; checked 2026/11/14). `/api/user/id/{id}` gives the counts,
//! `/api/stats/user/{role}/id/{id}?per_page=&after=` one page of channels with since-dates.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::http::json;
use crate::{Http, Result};

const ROLES: &str = "https://roles.tv/api";
/// The longest cursor sent back (roles.tv's are short base64).
const MAX_CURSOR: usize = 4096;

/// The four roles roles.tv lists, in the order the card shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OtherRole {
    Mod,
    Vip,
    Founder,
    Artist,
}

impl OtherRole {
    fn path(self) -> &'static str {
        match self {
            Self::Mod => "moderators",
            Self::Vip => "vips",
            Self::Founder => "founders",
            Self::Artist => "artists",
        }
    }
}

/// How many channels give them each role.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleCounts {
    pub mods: u32,
    pub vips: u32,
    pub founders: u32,
    pub artists: u32,
}

/// One channel where they hold the role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleChannel {
    pub id: String,
    pub login: String,
    pub name: String,
    /// RFC 3339 (UTC), when the role was given, if roles.tv knows.
    pub since: Option<String>,
    /// False once roles.tv saw it taken away.
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RolePage {
    pub channels: Vec<RoleChannel>,
    pub total: u32,
    /// The next page's cursor; none on the last one.
    pub next: Option<String>,
}

fn base(http: &Http) -> String {
    http.mock_base().map_or_else(|| ROLES.to_owned(), |b| format!("{b}/roles/api"))
}

/// A Twitch user id is digits only; anything else is never put in a URL.
fn plain_id(id: &str) -> Option<&str> {
    (!id.is_empty() && id.len() <= 32 && id.bytes().all(|b| b.is_ascii_digit())).then_some(id)
}

pub async fn counts(http: &Http, user_id: &str) -> Result<RoleCounts> {
    let Some(id) = plain_id(user_id) else { return Ok(RoleCounts::default()) };
    let raw: RawSummary = json(http.0.get(format!("{}/user/id/{id}", base(http)))).await?;
    Ok(raw.into())
}

pub async fn page(http: &Http, user_id: &str, role: OtherRole, per_page: u32, after: Option<&str>) -> Result<RolePage> {
    let Some(id) = plain_id(user_id) else {
        return Ok(RolePage { channels: Vec::new(), total: 0, next: None });
    };
    let mut req = http
        .0
        .get(format!("{}/stats/user/{}/id/{id}", base(http), role.path()))
        .query(&[("per_page", per_page.clamp(1, 100).to_string())]);
    if let Some(c) = after.filter(|c| !c.is_empty() && c.len() <= MAX_CURSOR) {
        req = req.query(&[("after", c)]);
    }
    let raw: RawPage = json(req).await?;
    Ok(read_page(raw, after))
}

/// roles.tv writes `2015-11-14 02:56:22.05279` (UTC, no zone); the app reads RFC 3339.
fn rfc3339(t: &str) -> Option<String> {
    let t = t.trim();
    if t.len() < 19 || !t.as_bytes()[..4].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut s = t.replacen(' ', "T", 1);
    let tail = s.get(19..).unwrap_or("");
    let zoned = s.ends_with('Z') || tail.contains('+') || tail.contains('-');
    if !zoned {
        s.push('Z');
    }
    Some(s)
}

fn read_page(raw: RawPage, asked: Option<&str>) -> RolePage {
    let channels = raw
        .data
        .into_iter()
        .filter(|c| !c.id.is_empty() && !c.login.is_empty())
        .map(|c| RoleChannel {
            name: c.display_name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| c.login.clone()),
            since: c.granted_at.as_deref().and_then(rfc3339),
            active: c.active.unwrap_or(true),
            id: c.id,
            login: c.login,
        })
        .collect::<Vec<_>>();
    // A cursor that repeats the one asked for, or an empty page, would loop.
    let next = raw
        .cursor
        .filter(|c| !c.is_empty() && c.len() <= MAX_CURSOR && Some(c.as_str()) != asked && !channels.is_empty());
    RolePage { total: raw.total.unwrap_or(0), channels, next }
}

#[derive(Deserialize)]
struct RawSummary {
    data: Option<RawSummaryData>,
}

#[derive(Deserialize)]
struct RawSummaryData {
    roles: Option<RawCounts>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawCounts {
    moderators: u32,
    vips: u32,
    founders: u32,
    artists: u32,
}

impl From<RawSummary> for RoleCounts {
    fn from(r: RawSummary) -> Self {
        let c = r.data.and_then(|d| d.roles).unwrap_or_default();
        Self { mods: c.moderators, vips: c.vips, founders: c.founders, artists: c.artists }
    }
}

#[derive(Deserialize)]
struct RawPage {
    total: Option<u32>,
    cursor: Option<String>,
    #[serde(default)]
    data: Vec<RawRoleChannel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRoleChannel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    login: String,
    display_name: Option<String>,
    granted_at: Option<String>,
    active: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// From real answers' shapes (2026/11/14, trimmed).
    #[test]
    fn v111_13_counts() {
        let raw: RawSummary = serde_json::from_str(
            r##"{"data":{"isBanned":false,"id":"23161357","login":"lirik","displayName":"LIRIK","chatColor":"#0029CC",
                "roles":{"artists":2,"founders":2,"moderators":77,"vips":15,"subscribers":0}}}"##,
        )
        .unwrap();
        assert_eq!(RoleCounts::from(raw), RoleCounts { mods: 77, vips: 15, founders: 2, artists: 2 });
        let none: RawSummary = serde_json::from_str(r#"{"data":null}"#).unwrap();
        assert_eq!(RoleCounts::from(none), RoleCounts::default());
    }

    #[test]
    fn v111_13_a_page() {
        let raw: RawPage = serde_json::from_str(
            r##"{"total":77,"page":1,"pages":26,"perPage":3,"cursor":"eyJpZCI6MX0=","data":[
                {"isBanned":false,"id":"36769016","login":"timthetatman","displayName":"TimTheTatman",
                 "grantedAt":"2015-11-14 02:56:22","active":true,"isPartner":true},
                {"id":"16764225","login":"syndicate","displayName":"","grantedAt":"2020-08-11 21:07:07.05279","active":false},
                {"id":"","login":"broken"}]}"##,
        )
        .unwrap();
        let p = read_page(raw, None);
        assert_eq!(p.total, 77);
        assert_eq!(p.next.as_deref(), Some("eyJpZCI6MX0="));
        assert_eq!(p.channels.len(), 2, "rows without an id are left out");
        assert_eq!(p.channels[0].name, "TimTheTatman");
        assert_eq!(p.channels[0].since.as_deref(), Some("2015-11-14T02:56:22Z"));
        assert_eq!(p.channels[1].name, "syndicate", "an empty display name falls back to the login");
        assert_eq!(p.channels[1].since.as_deref(), Some("2020-08-11T21:07:07.05279Z"));
        assert!(!p.channels[1].active);
    }

    #[test]
    fn v111_13_the_last_page_and_loops_end() {
        let last: RawPage =
            serde_json::from_str(r#"{"total":1,"cursor":null,"data":[{"id":"1","login":"a"}]}"#).unwrap();
        assert_eq!(read_page(last, None).next, None);
        let again: RawPage =
            serde_json::from_str(r#"{"total":9,"cursor":"c1","data":[{"id":"1","login":"a"}]}"#).unwrap();
        assert_eq!(read_page(again, Some("c1")).next, None, "the same cursor again stops");
        let empty: RawPage = serde_json::from_str(r#"{"total":9,"cursor":"c2","data":[]}"#).unwrap();
        assert_eq!(read_page(empty, Some("c1")).next, None, "an empty page stops");
    }

    #[test]
    fn v111_13_dates_and_ids() {
        assert_eq!(rfc3339("2026-05-02 23:18:35.187212").as_deref(), Some("2026-05-02T23:18:35.187212Z"));
        assert_eq!(rfc3339("2026-05-02T23:18:35Z").as_deref(), Some("2026-05-02T23:18:35Z"));
        assert_eq!(rfc3339("2026-05-02T23:18:35+02:00").as_deref(), Some("2026-05-02T23:18:35+02:00"));
        assert_eq!(rfc3339("soon"), None);
        assert_eq!(rfc3339(""), None);
        assert_eq!(plain_id("123"), Some("123"));
        assert_eq!(plain_id("1/../x"), None);
        assert_eq!(plain_id(""), None);
    }
}
