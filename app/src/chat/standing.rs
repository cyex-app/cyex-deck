//! V108-C40 The chat account's own standing in a channel, for the locked chat box: its ban or
//! timeout (with the reason), the channel's unban request rules and the last request, asking
//! for an unban, and acknowledging a warning. IRC tells a room about its own ban or timeout
//! (`RoomEvent::Restricted`); the account's `chatrooms-user-v1` topic adds unbans and
//! warnings while a chat box follows it. GQL: `cyex_twitch::gql::standing`.

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use cyex_twitch::gql::standing::{self as api, Change, Standing, Warning};
use cyex_twitch::hermes::{Event, Kind};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use crate::whispers::GqlCall;

pub const EVENT: &str = "chat:standing";

/// The account's standing in a channel moved; a warning comes with what it said.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StandingChanged {
    pub account: String,
    pub channel_id: String,
    pub warning: Option<Warning>,
}

fn call(app: &AppHandle, account: &str) -> Result<GqlCall, String> {
    GqlCall::new(app, account).map_err(|_| "This needs a GQL sign-in for the account (Settings → Accounts).".to_owned())
}

fn failed(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        return "Couldn’t reach Twitch.".into();
    }
    match e {
        cyex_twitch::Error::Refused(code) => {
            eprintln!("standing: refused {code}");
            api::explain(&code).to_owned()
        }
        e => {
            eprintln!("standing: {e}");
            "Twitch didn’t answer. Try again in a bit.".into()
        }
    }
}

/// The account's standing in `channel_id` now.
pub async fn read(app: &AppHandle, account: &str, channel_id: &str) -> Result<Standing, String> {
    #[cfg(feature = "mocks")]
    if let Some(s) = mock::read(account, channel_id) {
        return Ok(s);
    }
    let c = call(app, account)?;
    api::read(&c.http, c.auth(), channel_id, account).await.map_err(failed)
}

/// Only from the user's click on Send in the unban request form.
pub async fn request_unban(app: &AppHandle, account: &str, channel_id: &str, message: &str) -> Result<(), String> {
    let message = message.trim();
    if message.chars().count() > api::REQUEST_MAX {
        return Err("That message is too long.".into());
    }
    #[cfg(feature = "mocks")]
    if mock::request(account, channel_id, message) {
        return Ok(());
    }
    let c = call(app, account)?;
    api::request_unban(&c.http, c.auth(), channel_id, message).await.map_err(failed)
}

/// Only from the user's click on Acknowledge.
pub async fn acknowledge(app: &AppHandle, account: &str, channel_id: &str) -> Result<(), String> {
    #[cfg(feature = "mocks")]
    if mock::acknowledge(account, channel_id) {
        return Ok(());
    }
    let c = call(app, account)?;
    api::acknowledge_warning(&c.http, c.auth(), channel_id).await.map_err(failed)
}

/// Follows (or stops following) the account's own moderation topic while a chat box of it
/// is open; counted and held per window like the other user topics.
pub fn watch(app: &AppHandle, label: &str, account: &str, on: bool) {
    crate::hermes::hold(app, label, crate::hermes::Held::User(account.to_owned(), Kind::ChatroomsUser), on);
}

/// A message on the account's `chatrooms-user-v1` topic.
pub fn on_event(app: &AppHandle, e: &Event) {
    // Unverified shapes: the first sighting of each type is logged so a real one can be
    // checked (no names or tokens in it).
    let kind = e.payload.get("type").and_then(|t| t.as_str()).unwrap_or("?");
    if first_of(kind) {
        let keys: Vec<&str> = e.payload.get("data").and_then(|d| d.as_object()).map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default();
        eprintln!("standing: topic {kind}, data keys {keys:?}");
    }
    let Some((channel_id, change)) = api::change_of(&e.payload) else { return };
    let account = e.topic.channel_id.clone();
    match change {
        Change::Warned(w) => emit(app, &account, channel_id, Some(w)),
        Change::Moved => match gate(&mut MOVED.lock().unwrap_or_else(|p| p.into_inner()), (account.clone(), channel_id.clone()), Instant::now()) {
            Gate::Now => emit(app, &account, channel_id, None),
            // A burst reads once more at its end, not once per message.
            Gate::Later => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(MOVED_GAP).await;
                    MOVED.lock().unwrap_or_else(|p| p.into_inner()).remove(&(account.clone(), channel_id.clone()));
                    emit(&app, &account, channel_id, None);
                });
            }
            Gate::Skip => {}
        },
    }
}

/// V108-F56d Least time between two re-reads for one account and channel.
const MOVED_GAP: Duration = Duration::from_secs(1);

/// Per (account, channel): when it last passed, and whether a trailing read is booked.
type Moved = HashMap<(String, String), (Instant, bool)>;
static MOVED: LazyLock<Mutex<Moved>> = LazyLock::new(Mutex::default);
static SEEN: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Mutex::default);

fn first_of(kind: &str) -> bool {
    SEEN.lock().unwrap_or_else(|p| p.into_inner()).insert(kind.to_owned())
}

#[derive(Debug, PartialEq, Eq)]
enum Gate {
    /// Tell the page now.
    Now,
    /// Too soon: one read is booked for when the gap is over.
    Later,
    /// Too soon, and one is booked already.
    Skip,
}

fn gate(last: &mut Moved, key: (String, String), now: Instant) -> Gate {
    match last.get_mut(&key) {
        Some((at, booked)) if now.saturating_duration_since(*at) < MOVED_GAP => {
            if *booked {
                Gate::Skip
            } else {
                *booked = true;
                Gate::Later
            }
        }
        _ => {
            last.insert(key, (now, false));
            Gate::Now
        }
    }
}

fn emit(app: &AppHandle, account: &str, channel_id: String, warning: Option<Warning>) {
    let _ = app.emit(EVENT, StandingChanged { account: account.to_owned(), channel_id, warning });
}

/// Test builds only: a made-up standing per chat, so the locked box can be seen without a
/// real ban. Set from the page with `invoke('dev_standing', { account, channelId, kind })`.
#[cfg(feature = "mocks")]
pub mod mock {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};

    use cyex_twitch::gql::modlogs::{BanStatus, PastUnban};
    use cyex_twitch::gql::standing::{Standing, UnbanRules, Warning};
    use tauri::AppHandle;

    static MOCKS: LazyLock<Mutex<HashMap<(String, String), Standing>>> = LazyLock::new(Mutex::default);

    fn mocks() -> std::sync::MutexGuard<'static, HashMap<(String, String), Standing>> {
        MOCKS.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn key(account: &str, channel_id: &str) -> (String, String) {
        (account.to_owned(), channel_id.to_owned())
    }

    pub fn read(account: &str, channel_id: &str) -> Option<Standing> {
        mocks().get(&key(account, channel_id)).cloned()
    }

    /// A mocked chat takes the request and shows it pending.
    pub fn request(account: &str, channel_id: &str, message: &str) -> bool {
        let mut m = mocks();
        let Some(s) = m.get_mut(&key(account, channel_id)) else { return false };
        s.can_request = Some(false);
        s.request = Some(PastUnban {
            id: "mock-request".into(),
            created_at: rfc3339(0),
            text: message.to_owned(),
            status: "PENDING".into(),
            resolved_at: None,
            resolved_by: None,
            note: String::new(),
        });
        true
    }

    pub fn acknowledge(account: &str, channel_id: &str) -> bool {
        mocks().contains_key(&key(account, channel_id))
    }

    fn rfc3339(offset_secs: i64) -> String {
        let t = time::OffsetDateTime::now_utc() + time::Duration::seconds(offset_secs);
        t.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
    }

    fn ban(until: Option<String>, since_secs_ago: i64) -> BanStatus {
        BanStatus {
            until,
            by: Some("KuniModerator".into()),
            reason: "Spamming links after being asked to stop".into(),
            since: Some(rfc3339(-since_secs_ago)),
        }
    }

    /// `timeout` (5 min), `timeout-long` (1 h 2 min), `ban` (unban asks open in 2 h 15 min),
    /// `ban-ready` (open now), `ban-pending`, `ban-denied`, `ban-closed` (no requests),
    /// `warn` (also sends the warning event), `clear`.
    pub fn set(app: &AppHandle, account: &str, channel_id: &str, kind: &str) -> Result<(), String> {
        let rules = |on: bool| Some(UnbanRules { enabled: on, cooldown_minutes: 180 });
        let past = |status: &str, note: &str| PastUnban {
            id: "mock-request".into(),
            created_at: rfc3339(-3_600),
            text: "Sorry, it won't happen again.".into(),
            status: status.into(),
            resolved_at: (status != "PENDING").then(|| rfc3339(-600)),
            resolved_by: (status != "PENDING").then(|| "KuniModerator".to_owned()),
            note: note.into(),
        };
        let s = match kind {
            "timeout" => Standing { ban: Some(ban(Some(rfc3339(300)), 0)), ..Default::default() },
            "timeout-long" => Standing { ban: Some(ban(Some(rfc3339(3_720)), 0)), ..Default::default() },
            "ban" => Standing { ban: Some(ban(None, 45 * 60)), unban: rules(true), can_request: Some(false), request: None },
            "ban-ready" => Standing { ban: Some(ban(None, 4 * 3_600)), unban: rules(true), can_request: Some(true), request: None },
            "ban-pending" => Standing { ban: Some(ban(None, 4 * 3_600)), unban: rules(true), can_request: Some(false), request: Some(past("PENDING", "")) },
            "ban-denied" => Standing {
                ban: Some(ban(None, 4 * 3_600)),
                unban: rules(true),
                can_request: Some(true),
                request: Some(past("DENIED", "Not this time.")),
            },
            "ban-closed" => Standing { ban: Some(ban(None, 600)), unban: rules(false), can_request: Some(false), request: None },
            "warn" | "clear" => Standing::default(),
            _ => return Err(format!("unknown standing mock {kind}")),
        };
        mocks().insert(key(account, channel_id), s);
        let warning = (kind == "warn").then(|| Warning {
            reason: "Keep it friendly, please.".into(),
            rules: vec!["Be kind to each other".into(), "No spam".into()],
        });
        super::emit(app, account, channel_id.to_owned(), warning);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moved_reads_are_spaced_per_channel() {
        let (mut m, t0) = (Moved::new(), Instant::now());
        let k = |a: &str, c: &str| (a.to_owned(), c.to_owned());
        assert_eq!(gate(&mut m, k("1", "9"), t0), Gate::Now);
        assert_eq!(gate(&mut m, k("1", "9"), t0 + Duration::from_millis(200)), Gate::Later);
        assert_eq!(gate(&mut m, k("1", "9"), t0 + Duration::from_millis(400)), Gate::Skip);
        assert_eq!(gate(&mut m, k("1", "8"), t0 + Duration::from_millis(400)), Gate::Now, "another channel");
        assert_eq!(gate(&mut m, k("2", "9"), t0 + Duration::from_millis(400)), Gate::Now, "another account");
        assert_eq!(gate(&mut m, k("1", "9"), t0 + Duration::from_millis(1100)), Gate::Now);
    }

    #[test]
    fn a_warning_event_names_account_and_channel() {
        let e = StandingChanged { account: "1".into(), channel_id: "9".into(), warning: Some(Warning::default()) };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["channelId"], "9");
        assert!(v["warning"]["rules"].is_array());
    }
}
