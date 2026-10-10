//! 1.35 Ads. Running one is `commercial` in `roles` (`channel:edit:commercial`). This file
//! reads the ad schedule (`channel:read:ads`) and snoozes the next ad
//! (`channel:manage:ads`, `POST /channels/ads/schedule/snooze`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Auth, Data, get, post};
use crate::http::json;
use crate::{Http, Result};

/// When the next ad is due and how many snoozes are left.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export)]
pub struct AdSchedule {
    /// RFC 3339, or empty when none is scheduled.
    #[serde(default)]
    pub next_ad_at: String,
    #[serde(default)]
    pub last_ad_at: String,
    /// Length of the next ad, seconds.
    #[serde(default)]
    pub duration: u32,
    /// Ad-free seconds earned for pre-rolls.
    #[serde(default)]
    pub preroll_free_time: u32,
    /// Snoozes available now.
    #[serde(default)]
    pub snooze_count: u32,
    /// RFC 3339: when a snooze is added back.
    #[serde(default)]
    pub snooze_refresh_at: String,
}

pub async fn ad_schedule(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<AdSchedule> {
    let req = get(http, auth, "/channels/ads").query(&[("broadcaster_id", broadcaster_id)]);
    let d: Data<AdSchedule> = json(req).await?;
    Ok(d.data.into_iter().next().unwrap_or_default())
}

/// What a snooze leaves behind.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export)]
pub struct Snoozed {
    #[serde(default)]
    pub snooze_count: u32,
    #[serde(default)]
    pub snooze_refresh_at: String,
    #[serde(default)]
    pub next_ad_at: String,
}

/// Pushes the next ad back (`channel:manage:ads`). Errors when no snoozes are left.
pub async fn snooze(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<Snoozed> {
    let req = post(http, auth, "/channels/ads/schedule/snooze").query(&[("broadcaster_id", broadcaster_id)]);
    let d: Data<Snoozed> = json(req).await?;
    Ok(d.data.into_iter().next().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_parses() {
        let d: Data<AdSchedule> = serde_json::from_str(
            r#"{"data":[{"next_ad_at":"2024-01-01T12:00:00Z","last_ad_at":"2024-01-01T11:30:00Z",
            "duration":60,"preroll_free_time":0,"snooze_count":2,"snooze_refresh_at":"2024-01-01T12:15:00Z"}]}"#,
        )
        .unwrap();
        let a = d.data.into_iter().next().unwrap();
        assert_eq!((a.duration, a.snooze_count), (60, 2));
        assert_eq!(a.next_ad_at, "2024-01-01T12:00:00Z");
    }
}
