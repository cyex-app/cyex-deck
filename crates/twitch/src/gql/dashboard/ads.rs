//! 1.35 Ads on your own channel: run one, read the schedule, snooze the next one. The
//! schedule is the broadcaster's own read (`adProperties`); pre-roll-free time is public.

use serde::Deserialize;
use serde_json::json;

use super::Refusal;
use crate::gql::{Auth, query};
use crate::helix::ads::{AdSchedule, Snoozed};
use crate::random::hex;
use crate::{Error, Http, Result};

const START: &str = "mutation($input: StartAdInput!) { startAd(input: $input) { \
    adSession { lengthSeconds } error { code retryAfterSeconds } } }";
const SCHEDULE: &str = "query($id: ID!) { user(id: $id) { prerollFreeTimeSeconds adProperties { \
    lastTriggeredAt density { adSchedule { runAtTime durationSeconds } snooze { currentAmount refreshAt } } } } }";
const SNOOZE: &str = "mutation($input: SnoozeAdInput!) { snoozeAd(input: $input) { status } }";

/// Prefix of the refusal when Twitch wants a pause between ads; the seconds follow.
pub const RETRY_AFTER: &str = "RETRY_AFTER ";

/// Runs an ad; returns the length Twitch picked. A cooldown comes back as
/// `Refused("RETRY_AFTER <seconds>")`.
pub async fn start(http: &Http, auth: Auth<'_>, channel_id: &str, secs: u32) -> Result<u32> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Data {
        start_ad: Option<Payload>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Payload {
        ad_session: Option<AdSession>,
        error: Option<AdError>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AdSession {
        length_seconds: Option<u32>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AdError {
        code: Option<String>,
        retry_after_seconds: Option<u32>,
    }
    // `commercialID` keeps a resent request from running a second ad, as Twitch's own does.
    let input = json!({ "channelID": channel_id, "lengthSeconds": secs, "trigger": "QUICK_ACTION", "commercialID": hex(16) });
    let d: Data = query(http, auth, START, json!({ "input": input })).await?;
    let p = d.start_ad.ok_or(Error::Refused("UNKNOWN_ERROR".into()))?;
    if let Some(e) = p.error {
        if let Some(wait) = e.retry_after_seconds.filter(|s| *s > 0) {
            return Err(Error::Refused(format!("{RETRY_AFTER}{wait}")));
        }
        super::check(Some(Refusal { code: e.code }))?;
    }
    Ok(p.ad_session.and_then(|a| a.length_seconds).unwrap_or(secs))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Read {
    user: Option<User>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    preroll_free_time_seconds: Option<u32>,
    ad_properties: Option<Properties>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Properties {
    last_triggered_at: Option<String>,
    density: Option<Density>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Density {
    #[serde(default)]
    ad_schedule: Option<Vec<Slot>>,
    snooze: Option<Snooze>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Slot {
    run_at_time: Option<String>,
    duration_seconds: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snooze {
    current_amount: Option<u32>,
    refresh_at: Option<String>,
}

/// Twitch answers 1970-01-01 for "never"; the UI wants empty.
fn time(t: Option<&String>) -> String {
    t.filter(|t| !t.starts_with("1970-01-01"))
        .cloned()
        .unwrap_or_default()
}

fn schedule_from(r: Read) -> AdSchedule {
    let Some(user) = r.user else {
        return AdSchedule::default();
    };
    let props = user.ad_properties;
    let density = props.as_ref().and_then(|p| p.density.as_ref());
    let next = density
        .and_then(|d| d.ad_schedule.as_ref())
        .and_then(|s| s.first());
    let snooze = density.and_then(|d| d.snooze.as_ref());
    AdSchedule {
        next_ad_at: time(next.and_then(|n| n.run_at_time.as_ref())),
        last_ad_at: time(props.as_ref().and_then(|p| p.last_triggered_at.as_ref())),
        duration: next.and_then(|n| n.duration_seconds).unwrap_or_default(),
        preroll_free_time: user.preroll_free_time_seconds.unwrap_or_default(),
        snooze_count: snooze.and_then(|s| s.current_amount).unwrap_or_default(),
        snooze_refresh_at: time(snooze.and_then(|s| s.refresh_at.as_ref())),
    }
}

pub async fn schedule(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<AdSchedule> {
    let r: Read = query(http, auth, SCHEDULE, json!({ "id": channel_id })).await?;
    Ok(schedule_from(r))
}

/// Pushes the next ad back by Twitch's snooze length, then reads what's left.
pub async fn snooze(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Snoozed> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Data {
        snooze_ad: Option<Payload>,
    }
    #[derive(Deserialize)]
    struct Payload {
        status: Option<String>,
    }
    let d: Data = query(
        http,
        auth,
        SNOOZE,
        json!({ "input": { "targetChannelID": channel_id } }),
    )
    .await?;
    if d.snooze_ad.and_then(|p| p.status).as_deref() == Some("ERROR") {
        return Err(Error::Refused("SNOOZE_FAILED".into()));
    }
    let s = schedule(http, auth, channel_id).await?;
    Ok(Snoozed {
        snooze_count: s.snooze_count,
        snooze_refresh_at: s.snooze_refresh_at,
        next_ad_at: s.next_ad_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_reads_the_first_slot() {
        let r: Read = serde_json::from_str(
            r#"{"user":{"prerollFreeTimeSeconds":3314,"adProperties":{"lastTriggeredAt":"2026-10-11T10:00:00Z",
              "density":{"adSchedule":[{"runAtTime":"2026-10-11T11:00:00Z","durationSeconds":90},
              {"runAtTime":"2026-10-11T12:00:00Z","durationSeconds":60}],
              "snooze":{"currentAmount":2,"refreshAt":"2026-10-11T11:30:00Z"}}}}}"#,
        )
        .unwrap();
        let s = schedule_from(r);
        assert_eq!(s.next_ad_at, "2026-10-11T11:00:00Z");
        assert_eq!((s.duration, s.preroll_free_time, s.snooze_count), (90, 3314, 2));
        assert_eq!(s.last_ad_at, "2026-10-11T10:00:00Z");
        // Not an affiliate: nothing scheduled, and "never" reads as empty.
        let r: Read =
            serde_json::from_str(r#"{"user":{"prerollFreeTimeSeconds":0,"adProperties":null}}"#).unwrap();
        assert_eq!(schedule_from(r), AdSchedule::default());
        let r: Read = serde_json::from_str(
            r#"{"user":{"adProperties":{"lastTriggeredAt":"1970-01-01T00:00:00Z","density":{"adSchedule":[],
              "snooze":{"currentAmount":0,"refreshAt":"1970-01-01T00:00:00Z"}}}}}"#,
        )
        .unwrap();
        assert_eq!(schedule_from(r), AdSchedule::default());
    }
}
