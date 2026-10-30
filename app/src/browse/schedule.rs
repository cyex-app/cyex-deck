//! V108-N03 Channel schedules, read through GQL (anonymous). One channel's planned
//! streams for About and the .ics file; the next stream of many channels for the follow
//! cards, in a few batched calls and kept 15 minutes. Separate from `follows` on purpose.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cyex_twitch::Http;
use cyex_twitch::gql::schedule as gql_schedule;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::sync::Mutex;
use ts_rs::TS;

use super::follows::error_text;
use super::ics;
use crate::auth::Auth;

const NEXT_AGE: Duration = Duration::from_secs(15 * 60);
const CHANNEL_AGE: Duration = Duration::from_secs(5 * 60);
/// Streams About lists.
const SHOWN: usize = 10;
/// Ids per call, and calls at once.
const CHUNK: usize = 50;
const PARALLEL: usize = 3;
/// A stream with no end time counts as running this long.
const OPEN_ENDED: time::Duration = time::Duration::hours(3);

#[derive(Default)]
pub struct Schedules {
    next: Mutex<HashMap<String, Cached<Option<NextStream>>>>,
    channels: Mutex<HashMap<String, Cached<ChannelSchedule>>>,
}

struct Cached<T> {
    at: Instant,
    value: T,
}

/// One planned stream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScheduleSegment {
    pub id: String,
    pub title: String,
    /// RFC 3339, UTC.
    pub start: String,
    pub end: Option<String>,
    pub cancelled: bool,
    pub category: Option<String>,
}

/// No streams between the two times.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScheduleVacation {
    pub start: String,
    pub end: String,
    pub reason: String,
}

/// What About shows.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelSchedule {
    pub segments: Vec<ScheduleSegment>,
    pub vacation: Option<ScheduleVacation>,
}

/// A channel's next planned stream, by user id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NextStream {
    pub id: String,
    pub start: String,
    pub title: String,
    pub category: Option<String>,
}

impl From<gql_schedule::Segment> for ScheduleSegment {
    fn from(s: gql_schedule::Segment) -> Self {
        Self {
            id: s.id,
            title: s.title,
            start: s.start,
            end: s.end,
            cancelled: s.cancelled,
            category: s.category,
        }
    }
}

fn parse(t: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(t, &Rfc3339).ok()
}

/// The streams that are not over yet, soonest first, at most `max`. Cancelled ones stay
/// (flagged); ones whose time doesn't parse go.
fn upcoming(segments: Vec<ScheduleSegment>, now: OffsetDateTime, max: usize) -> Vec<ScheduleSegment> {
    let mut keep: Vec<(OffsetDateTime, ScheduleSegment)> = segments
        .into_iter()
        .filter_map(|s| {
            let start = parse(&s.start)?;
            let end = s.end.as_deref().and_then(parse).unwrap_or(start + OPEN_ENDED);
            (end > now).then_some((start, s))
        })
        .collect();
    keep.sort_by_key(|(start, _)| *start);
    keep.into_iter().map(|(_, s)| s).take(max).collect()
}

/// Cached entries still fresh, and the ids still to fetch. A next stream that has started
/// is stale whatever its age.
fn split(
    cache: &HashMap<String, Cached<Option<NextStream>>>,
    ids: &[String],
    now: Instant,
    clock: OffsetDateTime,
) -> (Vec<NextStream>, Vec<String>) {
    let (mut hit, mut miss) = (Vec::new(), Vec::new());
    for id in ids {
        let fresh = cache.get(id).filter(|c| {
            now.duration_since(c.at) < NEXT_AGE
                && c.value.as_ref().is_none_or(|n| parse(&n.start).is_some_and(|s| s > clock))
        });
        match fresh {
            Some(c) => hit.extend(c.value.clone()),
            None => miss.push(id.clone()),
        }
    }
    (hit, miss)
}

/// A channel's upcoming streams and vacation. Empty without a schedule.
pub async fn channel(app: &AppHandle, login: &str) -> Result<ChannelSchedule, String> {
    let login = login.trim().to_ascii_lowercase();
    let state = app.state::<Schedules>();
    if let Some(c) = state.channels.lock().await.get(&login)
        && c.at.elapsed() < CHANNEL_AGE
    {
        return Ok(c.value.clone());
    }
    let sched = fetch(app, &login).await?;
    let now = OffsetDateTime::now_utc();
    let mut value = ChannelSchedule {
        segments: upcoming(sched.segments.into_iter().map(ScheduleSegment::from).collect(), now, SHOWN),
        vacation: sched.vacation.map(|b| ScheduleVacation { start: b.start, end: b.end, reason: b.reason }),
    };
    // A vacation that's over isn't news.
    if value.vacation.as_ref().and_then(|v| parse(&v.end)).is_some_and(|e| e <= now) {
        value.vacation = None;
    }
    state
        .channels
        .lock()
        .await
        .insert(login, Cached { at: Instant::now(), value: value.clone() });
    Ok(value)
}

async fn fetch(app: &AppHandle, login: &str) -> Result<gql_schedule::Schedule, String> {
    let http = app.state::<Auth>().http.clone();
    gql_schedule::schedule(&http, login).await.map_err(error_text)
}

/// The next planned stream of each of `ids` that has one. Failed batches are left out
/// (and not cached), so the next ask tries them again.
pub async fn next(app: &AppHandle, ids: Vec<String>) -> Vec<NextStream> {
    let state = app.state::<Schedules>();
    let (mut found, missing) = split(&*state.next.lock().await, &ids, Instant::now(), OffsetDateTime::now_utc());
    if missing.is_empty() {
        return found;
    }
    let http = app.state::<Auth>().http.clone();
    let chunks: Vec<Vec<String>> = missing.chunks(CHUNK).map(<[String]>::to_vec).collect();
    let mut fetched: Vec<(Vec<String>, Vec<NextStream>)> = Vec::new();
    for group in chunks.chunks(PARALLEL) {
        let mut set = tokio::task::JoinSet::new();
        for ids in group.iter().cloned() {
            let http: Http = http.clone();
            set.spawn(async move {
                let got = gql_schedule::next_segments(&http, &ids).await;
                (ids, got)
            });
        }
        while let Some(Ok((ids, got))) = set.join_next().await {
            if let Ok(got) = got {
                let nexts = got
                    .into_iter()
                    .map(|(id, s)| NextStream { id, start: s.start, title: s.title, category: s.category })
                    .collect();
                fetched.push((ids, nexts));
            }
        }
    }
    let mut cache = state.next.lock().await;
    let now = Instant::now();
    for (asked, nexts) in fetched {
        for id in asked {
            let value = nexts.iter().find(|n| n.id == id).cloned();
            cache.insert(id, Cached { at: now, value });
        }
        found.extend(nexts);
    }
    found
}

/// The channel's schedule as .ics text, from a fresh read. `None` when there is nothing
/// to put in it.
pub async fn ics_text(app: &AppHandle, login: &str, name: &str) -> Result<Option<String>, String> {
    let sched = fetch(app, login.trim()).await?;
    let now = OffsetDateTime::now_utc();
    let segs = upcoming(sched.segments.into_iter().map(ScheduleSegment::from).collect(), now, usize::MAX);
    if segs.iter().all(|s| s.cancelled) {
        return Ok(None);
    }
    let events: Vec<ics::Event> = segs
        .iter()
        .map(|s| ics::Event {
            uid: &s.id,
            title: &s.title,
            category: s.category.as_deref(),
            start: &s.start,
            end: s.end.as_deref(),
            cancelled: s.cancelled,
        })
        .collect();
    Ok(Some(ics::calendar(name, &events, now)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(id: &str, start: &str, end: Option<&str>) -> ScheduleSegment {
        ScheduleSegment {
            id: id.into(),
            title: String::new(),
            start: start.into(),
            end: end.map(Into::into),
            cancelled: false,
            category: None,
        }
    }

    fn at(t: &str) -> OffsetDateTime {
        parse(t).unwrap()
    }

    #[test]
    fn upcoming_drops_finished_sorts_and_caps() {
        let now = at("2026-10-30T12:00:00Z");
        let got = upcoming(
            vec![
                seg("late", "2026-11-02T18:00:00Z", None),
                seg("over", "2026-10-30T08:00:00Z", Some("2026-10-30T10:00:00Z")),
                seg("now", "2026-10-30T11:00:00Z", Some("2026-10-30T14:00:00Z")),
                seg("bad", "tomorrow", None),
                seg("soon", "2026-10-31T18:00:00Z", None),
            ],
            now,
            3,
        );
        assert_eq!(got.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["now", "soon", "late"]);
        let two = vec![seg("a", "2026-10-31T18:00:00Z", None), seg("b", "2026-11-01T18:00:00Z", None)];
        assert_eq!(upcoming(two, now, 1).len(), 1);
    }

    #[test]
    fn an_open_ended_stream_counts_for_three_hours() {
        let s = vec![seg("a", "2026-10-30T10:00:00Z", None)];
        assert_eq!(upcoming(s.clone(), at("2026-10-30T12:59:00Z"), 5).len(), 1);
        assert!(upcoming(s, at("2026-10-30T13:01:00Z"), 5).is_empty());
    }

    #[test]
    fn split_reuses_fresh_entries_and_refetches_started_ones() {
        let clock = at("2026-10-30T12:00:00Z");
        let now = Instant::now();
        let next = |id: &str, start: &str| NextStream {
            id: id.into(),
            start: start.into(),
            title: String::new(),
            category: None,
        };
        let mut cache = HashMap::new();
        cache.insert("1".to_owned(), Cached { at: now, value: Some(next("1", "2026-10-30T20:00:00Z")) });
        cache.insert("2".to_owned(), Cached { at: now, value: None });
        cache.insert("3".to_owned(), Cached { at: now, value: Some(next("3", "2026-10-30T11:00:00Z")) });
        let ids: Vec<String> = ["1", "2", "3", "4"].map(String::from).into();
        let (hit, miss) = split(&cache, &ids, now, clock);
        assert_eq!(hit.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["1"]);
        assert_eq!(miss, ["3", "4"]);
        // Old entries are refetched.
        let (hit, miss) = split(&cache, &ids, now + NEXT_AGE, clock);
        assert!(hit.is_empty() && miss.len() == 4);
    }
}
