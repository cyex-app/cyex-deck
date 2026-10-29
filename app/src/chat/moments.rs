//! V108-N25 Moments and V108-N10 messages per second, from the rooms the app has joined.
//! The counting is `cyex_chat::moments`; this adds the stream's start time (so a moment
//! knows where in the broadcast it was) and the way out to the UI: an event for each new
//! moment, a command to list a channel's moments, one for its message rate.

use cyex_chat::moments::{Moment, Rate};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use ts_rs::TS;

use super::{Chat, now_ms};

const EVENT: &str = "chat:moments";

/// New moments found in a channel's chat.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MomentsFound {
    /// Channel login.
    pub channel: String,
    pub moments: Vec<Moment>,
}

/// Unix ms of an RFC 3339 time.
fn parse_ms(s: &str) -> Option<i64> {
    let t = OffsetDateTime::parse(s.trim(), &Rfc3339).ok()?;
    Some((t.unix_timestamp_nanos() / 1_000_000) as i64)
}

/// When the channel's current stream started, from the last live poll.
fn started_ms(app: &AppHandle, login: &str) -> Option<i64> {
    let live = crate::tabs::live::snapshot(app);
    let info = live.iter().find(|c| c.login == login)?;
    parse_ms(&info.stream.as_ref()?.started_at)
}

/// Seconds into the broadcast at `at_ms`; none before it began.
fn stream_secs(started: Option<i64>, at_ms: i64) -> Option<i64> {
    started.map(|s| (at_ms - s).div_euclid(1000)).filter(|s| *s >= 0)
}

fn fill(moments: &mut [Moment], started: Option<i64>) {
    for m in moments {
        m.stream_secs = stream_secs(started, m.at_ms);
    }
}

/// The room just found these: tell the windows. Called outside the chat lock.
pub fn announce(app: &AppHandle, channel: &str, mut found: Vec<Moment>) {
    fill(&mut found, started_ms(app, channel));
    let _ = app.emit(EVENT, MomentsFound { channel: channel.to_owned(), moments: found });
}

/// Every moment kept for a channel, oldest first. Chats of the same channel under other
/// accounts see the same lines; the fullest one answers.
pub fn list(app: &AppHandle, login: &str) -> Vec<Moment> {
    let login = login.to_ascii_lowercase();
    let mut best = {
        let chat = app.state::<Chat>();
        let inner = chat.lock();
        inner
            .rooms
            .iter()
            .filter(|((_, l), _)| *l == login)
            .map(|(_, e)| e.room.moments())
            .max_by_key(Vec::len)
            .unwrap_or_default()
    };
    fill(&mut best, started_ms(app, &login));
    best
}

/// Messages per second in a channel's chat now.
pub fn rate(app: &AppHandle, login: &str) -> Rate {
    let login = login.to_ascii_lowercase();
    let now = now_ms();
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    inner
        .rooms
        .iter()
        .filter(|((_, l), _)| *l == login)
        .map(|(_, e)| e.room.rate(now))
        .max_by(|a, b| a.per_sec.total_cmp(&b.per_sec))
        .unwrap_or(Rate { per_sec: 0.0, peak: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_twitch_start_times() {
        assert_eq!(parse_ms("2026-10-29T12:00:00Z"), Some(1_793_275_200_000));
        assert_eq!(parse_ms("nonsense"), None);
    }

    #[test]
    fn seconds_into_the_stream() {
        assert_eq!(stream_secs(Some(1_000), 61_999), Some(60));
        assert_eq!(stream_secs(Some(5_000), 1_000), None);
        assert_eq!(stream_secs(None, 1_000), None);
    }
}
