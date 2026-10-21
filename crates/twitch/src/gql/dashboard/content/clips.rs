//! V107-R39 Clips manager: clips of your channel or clips you made, by period, order and a
//! search, and deleting them by slug. Same shapes as Twitch's `ContentClipsManager_User`
//! and the bulk delete's `deleteClips(input: {slugs})`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::gql::connection::page as connection_page;
use crate::gql::dashboard::Channel;
use crate::gql::{Clip, ClipPeriod};
use crate::{Error, Result};

/// One page; Twitch's manager asks for 20.
pub const PAGE: u32 = 50;
/// Slugs per delete request; more go in further requests, paced by the caller.
pub const DELETE_MAX: usize = 25;

const LIST: &str = "query ContentClips($login: String!, $limit: Int, $cursor: Cursor, $criteria: UserClipsInput) { \
    user(login: $login) { clips(first: $limit, after: $cursor, criteria: $criteria) { pageInfo { hasNextPage } \
    edges { cursor node { slug title viewCount createdAt durationSeconds thumbnailURL \
    broadcaster { login displayName } curator { login displayName } game { displayName } } } } } }";
/// Who clipped (or which channels) for a filter; Twitch's clips manager fills its people
/// filter from this. `TERM` is CURATOR or BROADCASTER.
const PEOPLE: &str = "query ContentClipPeople($login: String!, $criteria: UserClipsInput!) { user(login: $login) { \
    clipsAggregations(first: 100, criteria: {query: $criteria, terms: [TERM]}) { term \
    buckets { ... on ClipUserAggregationBucket { user { id login displayName } } } } } }";
/// People a search looks through at most.
const PEOPLE_MAX: usize = 20;
const DELETE: &str = "mutation DeleteClips($input: DeleteClipsInput!) { deleteClips(input: $input) { clips { slug } } }";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClipsOf {
    /// Clips of your channel, by anyone.
    Channel,
    /// Clips you made, in any channel.
    Mine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClipOrder {
    Views,
    Newest,
}

#[derive(Clone, Debug, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipFilter {
    pub of: ClipsOf,
    pub period: ClipPeriod,
    pub order: ClipOrder,
    /// Title or who clipped it; empty for all.
    #[serde(default)]
    pub search: String,
    /// From the page before, for the next one.
    #[serde(default)]
    #[ts(optional)]
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipPage {
    pub clips: Vec<Clip>,
    /// Set when there are more.
    pub cursor: Option<String>,
}

/// The `UserClipsInput` for a filter, as Twitch's manager builds it.
pub fn criteria(f: &ClipFilter) -> Value {
    let period = match f.period {
        ClipPeriod::Day => "LAST_DAY",
        ClipPeriod::Week => "LAST_WEEK",
        ClipPeriod::Month => "LAST_MONTH",
        ClipPeriod::All => "ALL_TIME",
    };
    let sort = match f.order {
        ClipOrder::Views => "VIEWS_DESC",
        ClipOrder::Newest => "CREATED_AT_DESC",
    };
    let role = match f.of {
        ClipsOf::Channel => "BROADCASTER",
        ClipsOf::Mine => "CURATOR",
    };
    let text = f.search.trim();
    json!({ "period": period, "sort": sort, "role": role, "freeText": if text.is_empty() { Value::Null } else { text.into() } })
}

/// One page of `login`'s clips (your own login: the manager works on your account). A
/// search matches titles (Twitch's `freeText`) and who clipped them (in "Made by me": the
/// channel), the second through the people Twitch lists for the filter; those come first.
pub async fn list(ch: &Channel<'_>, login: &str, f: &ClipFilter) -> Result<ClipPage> {
    if f.search.trim().is_empty() {
        return page(ch, login, criteria(f), f.cursor.as_deref()).await;
    }
    let at = match f.cursor.as_deref() {
        Some(c) => serde_json::from_str::<Resume>(c).map_err(Error::Decode)?,
        None => {
            let ids = people(ch, login, f).await?;
            Resume { title: Some(String::new()), people: (!ids.is_empty()).then(String::new), ids }
        }
    };
    let mut clips = Vec::new();
    let mut next = Resume { ids: at.ids.clone(), ..Resume::default() };
    if let Some(c) = &at.people {
        let mut by = criteria(f);
        by["freeText"] = Value::Null;
        by[match f.of {
            ClipsOf::Channel => "curatorIDs",
            ClipsOf::Mine => "broadcasterIDs",
        }] = json!(at.ids);
        let p = page(ch, login, by, Some(c.as_str()).filter(|c| !c.is_empty())).await?;
        clips.extend(p.clips);
        next.people = p.cursor;
    }
    if let Some(c) = &at.title {
        let p = page(ch, login, criteria(f), Some(c.as_str()).filter(|c| !c.is_empty())).await?;
        clips.extend(p.clips);
        next.title = p.cursor;
    }
    let mut seen = std::collections::HashSet::new();
    clips.retain(|c: &Clip| seen.insert(c.slug.clone()));
    let more = next.title.is_some() || next.people.is_some();
    Ok(ClipPage { clips, cursor: more.then(|| json!(next).to_string()) })
}

/// Where a search goes on: each half's cursor (`None`: done; empty: from the top).
#[derive(Default, Serialize, Deserialize)]
struct Resume {
    title: Option<String>,
    people: Option<String>,
    ids: Vec<String>,
}

async fn page(ch: &Channel<'_>, login: &str, criteria: Value, cursor: Option<&str>) -> Result<ClipPage> {
    let vars = json!({ "login": login, "limit": PAGE, "cursor": cursor, "criteria": criteria });
    read_list(ch.read(LIST, vars).await?)
}

/// Ids of the people (clippers, or channels in "Made by me") whose name has the search in it.
async fn people(ch: &Channel<'_>, login: &str, f: &ClipFilter) -> Result<Vec<String>> {
    let term = match f.of {
        ClipsOf::Channel => "CURATOR",
        ClipsOf::Mine => "BROADCASTER",
    };
    let mut c = criteria(f);
    c["freeText"] = Value::Null;
    let text = PEOPLE.replace("TERM", term);
    let data = ch.read(&text, json!({ "login": login, "criteria": c })).await?;
    Ok(matching(data, &f.search))
}

pub(crate) fn matching(data: Value, search: &str) -> Vec<String> {
    let want = search.trim().trim_start_matches('@').to_lowercase();
    let has = |v: &Value| v.as_str().is_some_and(|s| s.to_lowercase().contains(&want));
    let mut out = Vec::new();
    for agg in data["user"]["clipsAggregations"].as_array().into_iter().flatten() {
        for b in agg["buckets"].as_array().into_iter().flatten() {
            let u = &b["user"];
            if let Some(id) = u["id"].as_str()
                && !want.is_empty()
                && (has(&u["login"]) || has(&u["displayName"]))
                && !out.iter().any(|x| x == id)
            {
                out.push(id.to_owned());
            }
        }
    }
    out.truncate(PEOPLE_MAX);
    out
}

pub(crate) fn read_list(data: Value) -> Result<ClipPage> {
    let conn = data.pointer("/user/clips").unwrap_or(&Value::Null);
    let mut bad = None;
    let p = connection_page(conn, |e| {
        let n = e.get("node").filter(|n| !n.is_null())?;
        serde_json::from_value::<RawClip>(n.clone()).map_err(|e| bad = Some(e)).ok().map(Clip::from)
    });
    if let Some(e) = bad {
        return Err(Error::Decode(e));
    }
    Ok(ClipPage { clips: p.items, cursor: p.next })
}

/// Deletes up to [`DELETE_MAX`] clips. Returns the slugs Twitch says it deleted.
pub async fn delete(ch: &Channel<'_>, slugs: &[String]) -> Result<Vec<String>> {
    if slugs.is_empty() {
        return Ok(Vec::new());
    }
    if slugs.len() > DELETE_MAX {
        return Err(Error::Refused("TOO_MANY".into()));
    }
    read_delete(ch.write(DELETE, "deleteClips", json!({ "slugs": slugs })).await?)
}

/// The slugs a `deleteClips` payload lists.
pub(crate) fn read_delete(payload: Value) -> Result<Vec<String>> {
    let clips = payload.get("clips").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    Ok(clips.iter().filter_map(|c| c.get("slug")?.as_str().map(str::to_owned)).collect())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawClip {
    slug: String,
    title: Option<String>,
    view_count: Option<u32>,
    created_at: Option<String>,
    duration_seconds: Option<u32>,
    #[serde(rename = "thumbnailURL")]
    thumbnail_url: Option<String>,
    broadcaster: Option<RawUser>,
    curator: Option<RawUser>,
    game: Option<RawGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    login: Option<String>,
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    display_name: Option<String>,
}

impl From<RawClip> for Clip {
    fn from(c: RawClip) -> Self {
        let (login, name) = c
            .broadcaster
            .map(|b| (b.login.unwrap_or_default(), b.display_name.unwrap_or_default()))
            .unwrap_or_default();
        let curator = c.curator.and_then(|u| u.display_name.filter(|n| !n.is_empty()).or(u.login));
        Clip {
            slug: c.slug,
            title: c.title.unwrap_or_default(),
            views: c.view_count.unwrap_or(0),
            created_at: c.created_at.unwrap_or_default(),
            duration: c.duration_seconds.unwrap_or(0),
            thumbnail: c.thumbnail_url.filter(|u| !u.is_empty()),
            channel_name: if name.is_empty() { login.clone() } else { name },
            channel_login: login,
            clipped_by: curator.filter(|n| !n.is_empty()),
            game: c.game.and_then(|g| g.display_name).filter(|n| !n.is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gql::dashboard::mock::{off, query, serve};
    use crate::gql::dashboard::NOT_DONE;
    use super::*;

    fn filter(of: ClipsOf, order: ClipOrder, search: &str) -> ClipFilter {
        ClipFilter { of, period: ClipPeriod::Week, order, search: search.into(), cursor: None }
    }

    #[test]
    fn criteria_match_twitchs_manager() {
        let c = criteria(&filter(ClipsOf::Channel, ClipOrder::Views, "  "));
        assert_eq!(c, json!({ "period": "LAST_WEEK", "sort": "VIEWS_DESC", "role": "BROADCASTER", "freeText": null }));
        let c = criteria(&filter(ClipsOf::Mine, ClipOrder::Newest, " gg "));
        assert_eq!(c["role"], "CURATOR");
        assert_eq!(c["sort"], "CREATED_AT_DESC");
        assert_eq!(c["freeText"], "gg");
    }

    #[test]
    fn reads_a_page_and_its_cursor() {
        let data = json!({ "user": { "clips": { "pageInfo": { "hasNextPage": true }, "edges": [
            { "cursor": "c1", "node": { "slug": "Fast-Clip", "title": "gg", "viewCount": 1200, "createdAt": "2026-10-20T10:00:00Z",
              "durationSeconds": 30, "thumbnailURL": "https://x/t.jpg", "broadcaster": { "login": "cyexbot", "displayName": "cyexbot" },
              "curator": { "login": "someone", "displayName": "Someone" }, "game": { "displayName": "Just Chatting" } } },
            { "cursor": "c2", "node": { "slug": "Bare", "title": null, "viewCount": null, "createdAt": null, "durationSeconds": null,
              "thumbnailURL": "", "broadcaster": null, "curator": { "login": "anon", "displayName": "" }, "game": null } },
            { "cursor": "c3", "node": null }
        ] } } });
        let p = read_list(data).unwrap();
        assert_eq!(p.cursor.as_deref(), Some("c3"));
        assert_eq!(p.clips.len(), 2);
        let a = &p.clips[0];
        assert_eq!((a.slug.as_str(), a.views, a.duration), ("Fast-Clip", 1200, 30));
        assert_eq!(a.clipped_by.as_deref(), Some("Someone"));
        assert_eq!(a.game.as_deref(), Some("Just Chatting"));
        let b = &p.clips[1];
        assert_eq!((b.title.as_str(), b.views, b.thumbnail.as_deref()), ("", 0, None));
        assert_eq!(b.clipped_by.as_deref(), Some("anon"));
    }

    #[test]
    fn last_page_has_no_cursor_and_no_user_is_empty() {
        let p = read_list(json!({ "user": { "clips": { "pageInfo": { "hasNextPage": false }, "edges": [{ "cursor": "c", "node": null }] } } })).unwrap();
        assert_eq!(p.cursor, None);
        assert_eq!(read_list(json!({ "user": null })).unwrap().clips, Vec::new());
    }

    #[tokio::test]
    async fn list_sends_the_filter() {
        let m = serve([r#"{"data":{"user":{"clips":{"pageInfo":{"hasNextPage":false},"edges":[]}}}}"#]);
        let mut f = filter(ClipsOf::Mine, ClipOrder::Newest, "");
        f.cursor = Some("abc".into());
        let p = list(&m.channel(), "cyexbot", &f).await.unwrap();
        assert!(p.clips.is_empty());
        let sent = &m.sent()[0];
        assert!(sent["query"].as_str().unwrap().contains("clips(first: $limit, after: $cursor, criteria: $criteria)"));
        assert_eq!(sent["variables"]["login"], "cyexbot");
        assert_eq!(sent["variables"]["cursor"], "abc");
        assert_eq!(sent["variables"]["criteria"]["role"], "CURATOR");
        assert_eq!(sent["variables"]["criteria"]["freeText"], Value::Null);
    }

    const PEOPLE_REPLY: &str = r#"{"data":{"user":{"clipsAggregations":[{"term":"CURATOR","buckets":[
        {"user":{"id":"11","login":"bossfan","displayName":"BossFan"}},{"user":{"id":"12","login":"other","displayName":"Other"}}]}]}}}"#;
    const BY_PEOPLE: &str = r#"{"data":{"user":{"clips":{"pageInfo":{"hasNextPage":true},"edges":[
        {"cursor":"p1","node":{"slug":"A","title":"x"}},{"cursor":"p2","node":{"slug":"B","title":"boss"}}]}}}}"#;
    const BY_TITLE: &str = r#"{"data":{"user":{"clips":{"pageInfo":{"hasNextPage":false},"edges":[
        {"cursor":"t1","node":{"slug":"B","title":"boss"}},{"cursor":"t2","node":{"slug":"C","title":"Boss down"}}]}}}}"#;
    const EMPTY: &str = r#"{"data":{"user":{"clips":{"pageInfo":{"hasNextPage":false},"edges":[]}}}}"#;

    #[tokio::test]
    async fn a_search_finds_titles_and_who_clipped() {
        let m = serve([PEOPLE_REPLY, BY_PEOPLE, BY_TITLE, EMPTY]);
        let mut f = filter(ClipsOf::Channel, ClipOrder::Views, " Boss ");
        let p = list(&m.channel(), "cyexbot", &f).await.unwrap();
        let slugs: Vec<&str> = p.clips.iter().map(|c| c.slug.as_str()).collect();
        assert_eq!(slugs, ["A", "B", "C"]);
        // Only the people half has more; the next page asks for that alone.
        f.cursor = p.cursor;
        let p = list(&m.channel(), "cyexbot", &f).await.unwrap();
        assert!(p.clips.is_empty() && p.cursor.is_none());
        let sent = m.sent();
        assert!(sent[0]["query"].as_str().unwrap().contains("terms: [CURATOR]"));
        assert_eq!(sent[0]["variables"]["criteria"]["freeText"], Value::Null);
        assert_eq!(sent[1]["variables"]["criteria"]["curatorIDs"], json!(["11"]));
        assert_eq!(sent[1]["variables"]["criteria"]["freeText"], Value::Null);
        assert_eq!(sent[2]["variables"]["criteria"]["freeText"], "Boss");
        assert_eq!(sent[3]["variables"]["cursor"], "p2");
        assert_eq!(sent[3]["variables"]["criteria"]["curatorIDs"], json!(["11"]));
    }

    #[tokio::test]
    async fn made_by_me_searches_channels() {
        let m = serve([r#"{"data":{"user":{"clipsAggregations":[]}}}"#, EMPTY]);
        let f = filter(ClipsOf::Mine, ClipOrder::Newest, "omlet");
        let p = list(&m.channel(), "cyexbot", &f).await.unwrap();
        assert!(p.clips.is_empty() && p.cursor.is_none());
        let sent = m.sent();
        assert!(sent[0]["query"].as_str().unwrap().contains("terms: [BROADCASTER]"));
        // Nobody matched: only the title search ran.
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[1]["variables"]["criteria"]["freeText"], "omlet");
    }

    #[test]
    fn people_match_by_login_or_name() {
        let data: Value = serde_json::from_str::<Value>(PEOPLE_REPLY).unwrap()["data"].clone();
        assert_eq!(matching(data.clone(), "@bossf"), vec!["11".to_owned()]);
        assert_eq!(matching(data.clone(), "OTH"), vec!["12".to_owned()]);
        assert!(matching(data.clone(), "zzz").is_empty());
        assert!(matching(data, " ").is_empty());
        assert!(matching(json!({ "user": null }), "a").is_empty());
    }

    #[tokio::test]
    async fn delete_sends_slugs_and_reads_what_went() {
        let m = serve([r#"{"data":{"deleteClips":{"clips":[{"slug":"A"},null,{"slug":"B"}]}}}"#]);
        let slugs = vec!["A".to_owned(), "B".to_owned()];
        let gone = delete(&m.channel(), &slugs).await.unwrap();
        assert_eq!(gone, slugs);
        let sent = &m.sent()[0];
        assert!(sent["query"].as_str().unwrap().starts_with("mutation DeleteClips($input: DeleteClipsInput!)"));
        assert_eq!(sent["variables"], json!({ "input": { "slugs": ["A", "B"] } }));
    }

    #[tokio::test]
    async fn delete_errors_come_back() {
        let m = serve([r#"{"errors":[{"message":"forbidden"}],"data":{"deleteClips":null}}"#]);
        let e = delete(&m.channel(), &["A".into()]).await.unwrap_err();
        assert_eq!(e.api_message(), Some("forbidden"));
        m.sent();
        // Too many at once never leaves the app; none at all sends nothing.
        let many: Vec<String> = (0..=DELETE_MAX).map(|i| i.to_string()).collect();
        assert!(matches!(delete(&off(), &many).await, Err(Error::Refused(_))));
        assert!(delete(&off(), &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn v107_r41_a_null_payload_is_not_done() {
        let m = serve([r#"{"data":{"deleteClips":null}}"#]);
        let e = delete(&m.channel(), &["A".into()]).await.unwrap_err();
        assert!(matches!(e, Error::Refused(c) if c == NOT_DONE));
        assert!(query(&m.sent()[0]).contains("deleteClips(input: $input)"));
    }
}
