//! 1.36 The usercard's outside data: chat logs and name history from logs.cyex.app (a
//! rustlog instance), and sub status from api.ivr.fi (D23), both public. C35 Chat history
//! comes from there too where the recent-messages service has none.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::http::json;
use crate::{Error, Http, Result};

const LOGS: &str = "https://logs.cyex.app";
const IVR: &str = "https://api.ivr.fi/v2/twitch";
/// Logs go back to before rustlog existed.
const FROM: &str = "2015-01-01T00:00:00Z";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NameChange {
    pub login: String,
    /// RFC 3339.
    pub first_seen: String,
    pub last_seen: String,
}

/// What a user's logs in a channel look like.
#[derive(Debug, PartialEq, Eq)]
pub enum Logs {
    /// Raw IRC lines, newest first.
    Lines(Vec<String>),
    None,
    /// The user asked rustlog not to keep their logs.
    OptedOut,
}

/// 1.36 Sub status in a channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SubStatus {
    pub months: u32,
    pub active: bool,
    /// `1`, `2`, `3` or `prime`, while active.
    pub tier: Option<String>,
    pub gift: bool,
    /// The user hides it; nothing else is known.
    pub hidden: bool,
}

/// The browser page with a user's logs in a channel.
pub fn page_url(channel: &str, user: &str) -> String {
    let enc = |s: &str| url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
    format!("{LOGS}/?c={}&u={}", enc(channel), enc(user))
}

pub async fn name_history(http: &Http, user_id: &str) -> Result<Vec<NameChange>> {
    let raw: Vec<RawName> = json(http.0.get(format!("{LOGS}/namehistory/{user_id}"))).await?;
    Ok(names(raw))
}

/// Newest first. The service sometimes lists a name that's empty; those are left out.
fn names(raw: Vec<RawName>) -> Vec<NameChange> {
    let mut out: Vec<NameChange> = raw
        .into_iter()
        .filter(|n| !n.user_login.trim().is_empty())
        .map(|n| NameChange {
            login: n.user_login,
            first_seen: n.first_timestamp,
            last_seen: n.last_timestamp,
        })
        .collect();
    out.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    out
}

/// The newest `limit` lines from `offset` on, by channel login and user id. `now` is
/// RFC 3339.
pub async fn user_logs(
    http: &Http,
    channel: &str,
    user_id: &str,
    limit: u32,
    offset: u32,
    now: &str,
) -> Result<Logs> {
    let req = http
        .0
        .get(format!("{LOGS}/channel/{channel}/userid/{user_id}"))
        .query(&[
            ("from", FROM),
            ("to", now),
            ("json", "1"),
            ("reverse", "1"),
            ("limit", &limit.to_string()),
            ("offset", &offset.to_string()),
        ]);
    match json::<RawLogs>(req).await {
        Ok(l) => Ok(Logs::Lines(l.messages.into_iter().map(|m| m.raw).collect())),
        Err(Error::Api { status: 404, .. }) => Ok(Logs::None),
        Err(Error::Api { status: 403, .. }) => Ok(Logs::OptedOut),
        Err(e) => Err(e),
    }
}

/// C35 A channel's newest `limit` lines between `from` and `to` (RFC 3339), oldest first.
/// Empty when the channel isn't logged or opted out.
pub async fn channel_lines(http: &Http, channel: &str, limit: u32, from: &str, to: &str) -> Result<Vec<String>> {
    let req = http.0.get(format!("{LOGS}/channel/{channel}")).query(&[
        ("from", from),
        ("to", to),
        ("json", "1"),
        ("reverse", "1"),
        ("limit", &limit.to_string()),
    ]);
    match json::<RawLogs>(req).await {
        Ok(l) => Ok(oldest_first(l)),
        Err(Error::Api { status: 403 | 404, .. }) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// `reverse=1` gives the newest lines first; chat history wants them the other way.
fn oldest_first(l: RawLogs) -> Vec<String> {
    l.messages.into_iter().rev().map(|m| m.raw).collect()
}

pub async fn sub_status(http: &Http, user: &str, channel: &str) -> Result<SubStatus> {
    let raw: RawSubAge = json(http.0.get(format!("{IVR}/subage/{user}/{channel}"))).await?;
    Ok(raw.into())
}

#[derive(Deserialize)]
struct RawName {
    user_login: String,
    first_timestamp: String,
    last_timestamp: String,
}

#[derive(Deserialize)]
struct RawLogs {
    #[serde(default)]
    messages: Vec<RawLine>,
}

#[derive(Deserialize)]
struct RawLine {
    raw: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSubAge {
    #[serde(default)]
    status_hidden: bool,
    cumulative: Option<RawCumulative>,
    meta: Option<RawMeta>,
}

#[derive(Deserialize)]
struct RawCumulative {
    months: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMeta {
    #[serde(rename = "type")]
    kind: Option<String>,
    tier: Option<String>,
    gift_meta: Option<serde_json::Value>,
}

impl From<RawSubAge> for SubStatus {
    fn from(r: RawSubAge) -> Self {
        let tier = r.meta.as_ref().map(|m| {
            if m.kind.as_deref() == Some("prime") {
                "prime".to_owned()
            } else {
                m.tier.clone().unwrap_or_else(|| "1".into())
            }
        });
        Self {
            months: r.cumulative.map_or(0, |c| c.months),
            active: r.meta.is_some(),
            gift: r
                .meta
                .as_ref()
                .is_some_and(|m| m.gift_meta.as_ref().is_some_and(|g| !g.is_null())),
            tier,
            hidden: r.status_hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_lines_come_oldest_first() {
        let l: RawLogs = serde_json::from_str(
            r#"{"messages":[{"raw":"@id=b :u!u@u PRIVMSG #c :new","text":"new"},{"raw":"@id=a :u!u@u PRIVMSG #c :old"}]}"#,
        )
        .unwrap();
        assert_eq!(oldest_first(l), ["@id=a :u!u@u PRIVMSG #c :old", "@id=b :u!u@u PRIVMSG #c :new"]);
    }

    #[test]
    fn sub_age() {
        let expired: RawSubAge = serde_json::from_str(
            r#"{"statusHidden":false,"followedAt":null,"streak":null,"cumulative":{"months":38},"meta":null}"#,
        )
        .unwrap();
        let s = SubStatus::from(expired);
        assert_eq!((s.months, s.active, s.tier), (38, false, None));
        let active: RawSubAge = serde_json::from_str(
            r#"{"statusHidden":false,"cumulative":{"months":5},"meta":{"type":"paid","tier":"2","giftMeta":{"giftDate":"x"}}}"#,
        )
        .unwrap();
        let s = SubStatus::from(active);
        assert!(s.active && s.gift);
        assert_eq!(s.tier.as_deref(), Some("2"));
        let never: RawSubAge =
            serde_json::from_str(r#"{"statusHidden":false,"cumulative":null,"meta":null}"#).unwrap();
        assert_eq!(SubStatus::from(never).months, 0);
    }

    #[test]
    fn empty_names_are_left_out() {
        let raw: Vec<RawName> = serde_json::from_str(
            r#"[{"user_login":"a","first_timestamp":"2024-01-01","last_timestamp":"2026-01-01"},
                {"user_login":"","first_timestamp":"2026-08-31","last_timestamp":"2026-09-05"},
                {"user_login":"b","first_timestamp":"2020-01-01","last_timestamp":"2023-01-01"}]"#,
        )
        .unwrap();
        let n = names(raw);
        assert_eq!(n.iter().map(|x| x.login.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn urls_and_logs() {
        assert_eq!(page_url("xqc", "forsen"), "https://logs.cyex.app/?c=xqc&u=forsen");
        let l: RawLogs =
            serde_json::from_str(r#"{"messages":[{"text":"hi","raw":"@id=1 :a!a@a PRIVMSG #xqc :hi"}]}"#)
                .unwrap();
        assert_eq!(l.messages[0].raw, "@id=1 :a!a@a PRIVMSG #xqc :hi");
    }
}
