//! V108-N03 `GET /schedule`: a channel's planned streams. Public data, any user token
//! works. 404 when the channel has no schedule. GQL (`gql::schedule`) is the batch route
//! for many channels; this one is the documented fallback for one.

use serde::Deserialize;

use super::{Auth, get};
use crate::http::json;
use crate::{Error, Http, Result};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Segment {
    pub id: String,
    /// RFC 3339.
    pub start_time: String,
    pub end_time: Option<String>,
    #[serde(default)]
    pub title: String,
    /// Set when this stream is skipped; the time it is back to normal.
    pub canceled_until: Option<String>,
    pub category: Option<Category>,
    #[serde(default)]
    pub is_recurring: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Vacation {
    pub start_time: String,
    pub end_time: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Schedule {
    /// `null` when the channel has only a vacation set.
    #[serde(default, deserialize_with = "list_or_empty")]
    pub segments: Vec<Segment>,
    pub vacation: Option<Vacation>,
}

fn list_or_empty<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Vec<Segment>, D::Error> {
    Ok(Option::<Vec<Segment>>::deserialize(d)?.unwrap_or_default())
}

#[derive(Deserialize)]
struct Answer {
    data: Option<Schedule>,
}

/// Up to `first` (1..=25) planned streams from now. Empty when the channel has no schedule.
pub async fn schedule(http: &Http, auth: Auth<'_>, broadcaster_id: &str, first: u8) -> Result<Schedule> {
    let req = get(http, auth, "/schedule")
        .query(&[("broadcaster_id", broadcaster_id)])
        .query(&[("first", first.clamp(1, 25))]);
    match json::<Answer>(req).await {
        Ok(a) => Ok(a.data.unwrap_or_default()),
        Err(Error::Api { status: 404, .. }) => Ok(Schedule::default()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_parses_with_a_vacation_and_a_skipped_stream() {
        let a: Answer = serde_json::from_str(
            r#"{"data":{"segments":[
              {"id":"e","start_time":"2026-10-30T18:00:00Z","end_time":"2026-10-30T22:00:00Z","title":"Ranked",
               "canceled_until":null,"category":{"id":"509658","name":"Just Chatting"},"is_recurring":true},
              {"id":"f","start_time":"2026-10-31T18:00:00Z","end_time":null,"title":"","canceled_until":"2026-11-01T00:00:00Z",
               "category":null,"is_recurring":false}],
             "broadcaster_id":"1","broadcaster_name":"X","broadcaster_login":"x",
             "vacation":{"start_time":"2026-11-02T00:00:00Z","end_time":"2026-11-09T00:00:00Z"}},"pagination":{}}"#,
        )
        .unwrap();
        let s = a.data.unwrap();
        assert_eq!(s.segments.len(), 2);
        assert_eq!(s.segments[0].category.as_ref().unwrap().name, "Just Chatting");
        assert!(s.segments[1].canceled_until.is_some() && s.segments[1].category.is_none());
        assert_eq!(s.vacation.unwrap().end_time, "2026-11-09T00:00:00Z");
    }

    #[test]
    fn a_null_segment_list_is_empty() {
        let a: Answer = serde_json::from_str(
            r#"{"data":{"segments":null,"vacation":{"start_time":"a","end_time":"b"}}}"#,
        )
        .unwrap();
        assert!(a.data.unwrap().segments.is_empty());
    }
}
