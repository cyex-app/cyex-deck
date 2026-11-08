//! V110-U43 The raid banner over a chat: a raid counting down from the channel the chat
//! shows, read from the same Hermes topic as the stream manager's countdown (D2,
//! `raid.<channel id>`). Followed per chat view while it is open (counted, so the chats of a
//! channel share it), for that channel only. When the raid goes, one line in its chats.
//! Joining is only ever by hand (`join`), through GQL `joinRaid` like the miner.

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

use cyex_chat::model::NoticeKind;
use cyex_core::number::grouped;
use cyex_twitch::gql::dashboard::raids::{RaidEvent, RaidStatus, parse_raid};
use cyex_twitch::hermes::{Event, Kind};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::Chat;

const EVENT: &str = "chat:raid";
/// Ended raid ids kept, so a late update or the repeated go messages (Twitch sends about
/// five) don't bring the banner back or add a second line.
const ENDED_KEPT: usize = 50;
/// A raid past its time by this much is stale (its go message was missed).
const STALE_SECS: i64 = 30;

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RaidChanged {
    pub channel_id: String,
    /// The raid counting down, `None` when it went or was called off.
    pub raid: Option<RaidStatus>,
    /// It went (else it was called off or is still counting down).
    pub went: bool,
}

/// What a topic message does to the banner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Keep,
    Show(RaidStatus),
    /// It went: the banner goes and the chat gets a line.
    Went(RaidStatus),
    /// Called off.
    Gone,
}

#[derive(Default)]
pub struct Raids {
    by_channel: HashMap<String, RaidStatus>,
    ended: VecDeque<String>,
}

impl Raids {
    /// The banner's state machine: start and update show it, go and cancel end it once.
    pub fn step(&mut self, channel_id: &str, ev: RaidEvent, went: bool) -> Step {
        match ev {
            RaidEvent::Update(r) => {
                if self.ended.contains(&r.id) {
                    return Step::Keep;
                }
                if self.by_channel.get(channel_id) == Some(&r) {
                    return Step::Keep;
                }
                self.by_channel.insert(channel_id.to_owned(), r.clone());
                Step::Show(r)
            }
            RaidEvent::Done { id } => {
                if !self.ended.contains(&id) {
                    self.ended.push_back(id.clone());
                    if self.ended.len() > ENDED_KEPT {
                        self.ended.pop_front();
                    }
                }
                let Some(cur) = self.by_channel.get(channel_id).filter(|c| c.id == id).cloned() else {
                    return Step::Keep;
                };
                self.by_channel.remove(channel_id);
                if went { Step::Went(cur) } else { Step::Gone }
            }
        }
    }

    pub fn current(&self, channel_id: &str, now: i64) -> Option<RaidStatus> {
        self.by_channel.get(channel_id).filter(|r| r.goes_at + STALE_SECS > now).cloned()
    }

    pub fn forget(&mut self, channel_id: &str) {
        self.by_channel.remove(channel_id);
    }
}

static RAIDS: LazyLock<Mutex<Raids>> = LazyLock::new(Mutex::default);

fn raids() -> std::sync::MutexGuard<'static, Raids> {
    RAIDS.lock().unwrap_or_else(|p| p.into_inner())
}

/// The chat line when it goes: "Raided Kuni with 1.234 viewers".
pub fn went_line(r: &RaidStatus) -> String {
    let n = r.viewers;
    format!("Raided {} with {} {}", r.target_name, grouped(n), if n == 1 { "viewer" } else { "viewers" })
}

/// The raid counting down from the channel, for a chat that opens mid-raid.
pub fn current(channel_id: &str) -> Option<RaidStatus> {
    raids().current(channel_id, crate::auth::unix_now())
}

/// Follows (or stops following) the channel's raid topic while a chat of it shows. Held per
/// window (`label`), so a page that goes away without its cleanup lets go too.
pub fn watch(app: &AppHandle, label: &str, channel_id: &str, on: bool) {
    if channel_id.is_empty() {
        return;
    }
    let held = crate::hermes::Held::Channel(Kind::Raid, channel_id.to_owned());
    crate::hermes::hold(app, label, held, on);
    if on {
        #[cfg(feature = "mocks")]
        super::raid_mock::start(app);
    } else if !crate::hermes::held(app, Kind::Raid, channel_id) {
        raids().forget(channel_id);
    }
}

/// A message of the topic (shared with the miner and the stream manager).
pub fn on_event(app: &AppHandle, e: &Event) {
    let id = &e.topic.channel_id;
    let Some(ev) = parse_raid(&e.payload, id) else { return };
    let went = e.payload["type"] == "raid_go_v2";
    let step = raids().step(id, ev, went);
    let (raid, went) = match step {
        Step::Keep => return,
        Step::Show(r) => (Some(r), false),
        Step::Gone => (None, false),
        Step::Went(r) => {
            if let Some(login) = app.state::<Chat>().channels.login_of(id) {
                super::notice_line(app, &login, NoticeKind::Raid, &went_line(&r));
            }
            (None, true)
        }
    };
    let _ = app.emit(EVENT, RaidChanged { channel_id: id.clone(), raid, went });
}

/// Twitch, or in test builds the raid mock when it is set.
fn http(app: &AppHandle) -> cyex_twitch::Http {
    let http = app.state::<crate::auth::Auth>().http.clone();
    #[cfg(feature = "mocks")]
    let http = super::raid_mock::http(app, http);
    http
}

/// Joins the raid as `account`, only ever from the banner's button.
pub async fn join(app: &AppHandle, account: &str, raid_id: &str) -> Result<(), String> {
    let pair = crate::auth::gql_token(app, account);
    #[cfg(feature = "mocks")]
    let pair = pair.or_else(|| super::raid_mock::fake_token(app));
    let Some((token, client_id)) = pair else {
        return Err("Sign in to join raids.".to_owned());
    };
    let auth = crate::auth::gql_auth(&token, &client_id);
    cyex_twitch::gql::miner::join_raid(&http(app), auth, raid_id).await.map_err(|e| {
        if e.is_network() {
            "Couldn’t reach Twitch.".to_owned()
        } else {
            eprintln!("raid join: {e}");
            "Couldn’t join the raid.".to_owned()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raid(id: &str, viewers: u64) -> RaidStatus {
        RaidStatus {
            id: id.into(),
            target_id: "9".into(),
            target_login: "kuni".into(),
            target_name: "Kuni".into(),
            viewers,
            created_at: 1000,
            goes_at: 1090,
            ..RaidStatus::default()
        }
    }

    #[test]
    fn start_update_go() {
        let mut r = Raids::default();
        assert_eq!(r.step("7", RaidEvent::Update(raid("a", 10)), false), Step::Show(raid("a", 10)));
        // The same message again changes nothing; a new count shows.
        assert_eq!(r.step("7", RaidEvent::Update(raid("a", 10)), false), Step::Keep);
        assert_eq!(r.step("7", RaidEvent::Update(raid("a", 1234)), false), Step::Show(raid("a", 1234)));
        assert_eq!(r.current("7", 1000), Some(raid("a", 1234)));
        assert_eq!(r.step("7", RaidEvent::Done { id: "a".into() }, true), Step::Went(raid("a", 1234)));
        // Twitch repeats the go; no second line, and a late update doesn't bring it back.
        assert_eq!(r.step("7", RaidEvent::Done { id: "a".into() }, true), Step::Keep);
        assert_eq!(r.step("7", RaidEvent::Update(raid("a", 1300)), false), Step::Keep);
        assert_eq!(r.current("7", 1000), None);
    }

    #[test]
    fn cancel_and_other_channels() {
        let mut r = Raids::default();
        r.step("7", RaidEvent::Update(raid("a", 5)), false);
        r.step("8", RaidEvent::Update(raid("b", 6)), false);
        // Another raid's end leaves this one.
        assert_eq!(r.step("7", RaidEvent::Done { id: "b".into() }, false), Step::Keep);
        assert_eq!(r.step("7", RaidEvent::Done { id: "a".into() }, false), Step::Gone);
        assert_eq!(r.current("8", 1000), Some(raid("b", 6)));
        // A new raid after a cancel shows.
        assert_eq!(r.step("7", RaidEvent::Update(raid("c", 1)), false), Step::Show(raid("c", 1)));
    }

    #[test]
    fn stale_raids_dont_show() {
        let mut r = Raids::default();
        r.step("7", RaidEvent::Update(raid("a", 5)), false);
        assert!(r.current("7", 1090 + STALE_SECS - 1).is_some());
        assert!(r.current("7", 1090 + STALE_SECS).is_none());
    }

    #[test]
    fn line_reads_well() {
        assert_eq!(went_line(&raid("a", 1234)), "Raided Kuni with 1.234 viewers");
        assert_eq!(went_line(&raid("a", 1)), "Raided Kuni with 1 viewer");
    }
}
