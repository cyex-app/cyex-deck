//! V108-N03 A channel's stream schedule, anonymous reads. Schema-checked 2026/10/29
//! (signed out, empty login): `Channel.schedule { id segments nextSegment interruption }`;
//! `segments` takes no arguments; `ScheduleSegment { id title startAt endAt isCancelled
//! hasReminder categories { id displayName } }`; `interruption { startAt endAt reason }` is
//! the vacation. `schedule` is null for a channel without one. `users(ids:)` batches the
//! follow list's "next stream" in a few calls.

use serde::Deserialize;
use serde_json::json;

use super::{Auth, query};
use crate::{Http, Result};

/// Ids per call; Twitch fails big `users(ids:)` calls now and then.
const BATCH: usize = 50;

const SEGMENT: &str = "id title startAt endAt isCancelled categories { id displayName }";

/// One planned stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub id: String,
    pub title: String,
    /// RFC 3339.
    pub start: String,
    /// RFC 3339.
    pub end: Option<String>,
    pub cancelled: bool,
    pub category: Option<String>,
}

/// A vacation: no streams between the two times.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Break {
    pub start: String,
    pub end: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schedule {
    pub segments: Vec<Segment>,
    pub vacation: Option<Break>,
}

/// The channel's planned streams (cancelled ones included, flagged), soonest first.
/// Empty when the channel has no schedule.
pub async fn schedule(http: &Http, login: &str) -> Result<Schedule> {
    let q = format!(
        "query($login: String!) {{ user(login: $login) {{ channel {{ schedule {{ segments {{ {SEGMENT} }} \
         interruption {{ startAt endAt reason }} }} }} }} }}"
    );
    let data: One = query(http, Auth::PUBLIC, &q, json!({ "login": login })).await?;
    Ok(data.user.and_then(|u| u.channel).and_then(|c| c.schedule).map(Schedule::from).unwrap_or_default())
}

/// The next planned, not cancelled stream of each channel that has one: `(user id, segment)`.
/// Unknown channels and ones without a schedule are left out. In chunks of [`BATCH`].
pub async fn next_segments(http: &Http, ids: &[String]) -> Result<Vec<(String, Segment)>> {
    let q = format!("query($ids: [ID!]) {{ users(ids: $ids) {{ id channel {{ schedule {{ nextSegment {{ {SEGMENT} }} }} }} }} }}");
    let mut out = Vec::new();
    for chunk in ids.chunks(BATCH) {
        let data: Many = query(http, Auth::PUBLIC, &q, json!({ "ids": chunk })).await?;
        out.extend(data.users.into_iter().flatten().filter_map(next_of));
    }
    Ok(out)
}

fn next_of(u: ManyUser) -> Option<(String, Segment)> {
    let seg = Segment::from(u.channel?.schedule?.next_segment?);
    (!seg.cancelled && !seg.start.is_empty()).then_some((u.id, seg))
}

impl From<RawSchedule> for Schedule {
    fn from(s: RawSchedule) -> Self {
        let mut segments: Vec<Segment> =
            s.segments.into_iter().flatten().flatten().map(Segment::from).filter(|s| !s.start.is_empty()).collect();
        // RFC 3339 in UTC sorts as text.
        segments.sort_by(|a, b| a.start.cmp(&b.start));
        Self {
            segments,
            vacation: s
                .interruption
                .and_then(|i| Some(Break { start: i.start_at?, end: i.end_at?, reason: i.reason.unwrap_or_default() })),
        }
    }
}

impl From<RawSegment> for Segment {
    fn from(s: RawSegment) -> Self {
        Self {
            id: s.id.unwrap_or_default(),
            title: s.title.unwrap_or_default(),
            start: s.start_at.unwrap_or_default(),
            end: s.end_at,
            cancelled: s.is_cancelled.unwrap_or(false),
            category: s.categories.into_iter().flatten().flatten().find_map(|c| c.display_name),
        }
    }
}

#[derive(Deserialize)]
struct One {
    user: Option<OneUser>,
}

#[derive(Deserialize)]
struct OneUser {
    channel: Option<Chan>,
}

#[derive(Deserialize)]
struct Many {
    users: Vec<Option<ManyUser>>,
}

#[derive(Deserialize)]
struct ManyUser {
    id: String,
    channel: Option<Chan>,
}

#[derive(Deserialize)]
struct Chan {
    schedule: Option<RawSchedule>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSchedule {
    /// V108-N03 Null as well as missing: `#[serde(default)]` alone fails on `null`.
    #[serde(default)]
    segments: Option<Vec<Option<RawSegment>>>,
    next_segment: Option<RawSegment>,
    interruption: Option<RawBreak>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSegment {
    id: Option<String>,
    title: Option<String>,
    start_at: Option<String>,
    end_at: Option<String>,
    is_cancelled: Option<bool>,
    categories: Option<Vec<Option<RawCategory>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCategory {
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBreak {
    start_at: Option<String>,
    end_at: Option<String>,
    reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_sorts_and_reads_the_vacation() {
        let d: One = serde_json::from_str(
            r#"{"user":{"channel":{"schedule":{"segments":[
              {"id":"b","title":null,"startAt":"2026-10-31T18:00:00Z","endAt":null,"isCancelled":true,"categories":[]},
              {"id":"a","title":"Ranked","startAt":"2026-10-30T18:00:00Z","endAt":"2026-10-30T22:00:00Z","isCancelled":false,
               "categories":[{"id":"1","displayName":"Just Chatting"}]},
              null],
              "nextSegment":null,"interruption":{"startAt":"2026-11-02T00:00:00Z","endAt":"2026-11-09T00:00:00Z","reason":"Trip"}}}}}"#,
        )
        .unwrap();
        let s = Schedule::from(d.user.unwrap().channel.unwrap().schedule.unwrap());
        assert_eq!(s.segments.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(s.segments[0].category.as_deref(), Some("Just Chatting"));
        assert!(s.segments[1].cancelled && s.segments[1].title.is_empty());
        assert_eq!(s.vacation.unwrap().reason, "Trip");
    }

    #[test]
    fn no_schedule_reads_empty() {
        let d: One = serde_json::from_str(r#"{"user":{"channel":{"schedule":null}}}"#).unwrap();
        assert!(d.user.and_then(|u| u.channel).and_then(|c| c.schedule).is_none());
        let d: One = serde_json::from_str(r#"{"user":null}"#).unwrap();
        assert!(d.user.is_none());
    }

    #[test]
    fn null_segments_read_empty() {
        let d: One = serde_json::from_str(
            r#"{"user":{"channel":{"schedule":{"segments":null,"nextSegment":null,"interruption":null}}}}"#,
        )
        .unwrap();
        let s = Schedule::from(d.user.unwrap().channel.unwrap().schedule.unwrap());
        assert_eq!(s, Schedule::default());
    }

    #[test]
    fn next_segments_skip_cancelled_and_missing() {
        let d: Many = serde_json::from_str(
            r#"{"users":[
              {"id":"1","channel":{"schedule":{"nextSegment":{"id":"s","title":"T","startAt":"2026-10-30T20:00:00Z","endAt":null,"isCancelled":false,"categories":null}}}},
              {"id":"2","channel":{"schedule":{"nextSegment":{"id":"c","title":"T","startAt":"2026-10-30T20:00:00Z","isCancelled":true}}}},
              {"id":"3","channel":{"schedule":null}},
              null]}"#,
        )
        .unwrap();
        let got: Vec<_> = d.users.into_iter().flatten().filter_map(next_of).collect();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "1");
        assert_eq!(got[0].1.category, None);
    }
}
