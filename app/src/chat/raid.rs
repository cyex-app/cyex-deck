//! V110-U43 The raid banner over a chat: a raid counting down from the channel the chat
//! shows, read from the same Hermes topic as the stream manager's countdown (D2,
//! `raid.<channel id>`). Followed per chat view while it is open (counted, so the chats of a
//! channel share it), for that channel only. When the raid goes, one line in its chats.
//! Joining is only ever by hand: ticking "Join" on the banner calls GQL `joinRaid` like the
//! miner (`join`), per raid id. Twitch has no leave call that we know of, so unticking only
//! drops the pop-up (`leave`). When the raid goes, the ticked raid's channel opens in a
//! pop-out of the main window (`chat:raid-go`); a cancelled raid forgets the tick.

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
/// A raid's tick changed (or was cleared by the raid going or being called off).
const JOIN_EVENT: &str = "chat:raid-join";
/// The ticked raid went: the main window opens the raided channel.
const GO_EVENT: &str = "chat:raid-go";
/// Ticked raids kept (a stale tick only costs this much).
const JOINED_KEPT: usize = 20;
/// Ended raid ids kept, so a late update or the repeated go messages (Twitch sends about
/// five) don't bring the banner back or add a second line.
const ENDED_KEPT: usize = 50;
/// A raid past its time by this much is stale (its go message was missed).
const STALE_SECS: i64 = 30;
/// V110-F72 How long a go nobody acknowledged is kept for a page that was reloading.
const PENDING_SECS: i64 = 60;

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

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RaidJoinChanged {
    pub raid_id: String,
    pub on: bool,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RaidGo {
    pub raid: RaidStatus,
    /// The account that ticked Join.
    pub account: String,
}

/// What a topic message does to the banner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Keep,
    Show(RaidStatus),
    /// It went: the banner goes and the chat gets a line.
    Went(RaidStatus),
    /// Called off.
    Gone(RaidStatus),
}

/// What ticking Join for a raid came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Tick {
    On,
    /// The raid had ended; the raid itself when it went.
    Ended(Option<RaidStatus>),
}

#[derive(Default)]
pub struct Raids {
    by_channel: HashMap<String, RaidStatus>,
    /// Raids that ended, and the raid itself when it went (a Join that finishes late opens it).
    ended: VecDeque<(String, Option<RaidStatus>)>,
    /// Raid id and the account that ticked Join, oldest first.
    joined: Vec<(String, String)>,
    /// V110-F72 Ticked raids that went, not yet acknowledged by the main page: when, raid,
    /// account. A page reloading while the go was emitted never heard it.
    pending: Vec<(i64, RaidStatus, String)>,
}

impl Raids {
    /// The banner's state machine: start and update show it, go and cancel end it once.
    pub fn step(&mut self, channel_id: &str, ev: RaidEvent, went: bool) -> Step {
        match ev {
            RaidEvent::Update(r) => {
                if self.is_ended(&r.id) {
                    return Step::Keep;
                }
                if self.by_channel.get(channel_id) == Some(&r) {
                    return Step::Keep;
                }
                self.by_channel.insert(channel_id.to_owned(), r.clone());
                Step::Show(r)
            }
            RaidEvent::Done { id } => {
                let cur = self.by_channel.get(channel_id).filter(|c| c.id == id).cloned();
                if !self.is_ended(&id) {
                    self.ended.push_back((id.clone(), cur.clone().filter(|_| went)));
                    if self.ended.len() > ENDED_KEPT {
                        self.ended.pop_front();
                    }
                }
                let Some(cur) = cur else {
                    return Step::Keep;
                };
                self.by_channel.remove(channel_id);
                if went { Step::Went(cur) } else { Step::Gone(cur) }
            }
        }
    }

    fn is_ended(&self, raid_id: &str) -> bool {
        self.ended.iter().any(|(id, _)| id == raid_id)
    }

    /// Drops banner state whose raid is long past (its end message was missed).
    pub fn purge(&mut self, now: i64) {
        self.by_channel.retain(|_, r| r.goes_at + STALE_SECS > now);
    }

    /// A Join that finished: ticked unless the raid already ended, checked in one go with the
    /// tick so a go in between can't leave a stale one.
    pub fn tick_live(&mut self, raid_id: &str, account: &str) -> Tick {
        if let Some((_, went)) = self.ended.iter().find(|(id, _)| id == raid_id) {
            return Tick::Ended(went.clone());
        }
        self.tick(raid_id, account);
        Tick::On
    }

    /// V110-N11 The update is the first we see of a raid that began just now (its line is due).
    pub fn starts(&self, channel_id: &str, ev: &RaidEvent, now: i64) -> bool {
        let RaidEvent::Update(r) = ev else { return false };
        !self.is_ended(&r.id)
            && self.by_channel.get(channel_id).is_none_or(|c| c.id != r.id)
            && now - r.created_at <= FRESH_SECS
    }

    pub fn current(&self, channel_id: &str, now: i64) -> Option<RaidStatus> {
        self.by_channel.get(channel_id).filter(|r| r.goes_at + STALE_SECS > now).cloned()
    }

    /// Join ticked for the raid by `account` (once per raid id).
    pub fn tick(&mut self, raid_id: &str, account: &str) {
        self.joined.retain(|(id, _)| id != raid_id);
        self.joined.push((raid_id.to_owned(), account.to_owned()));
        if self.joined.len() > JOINED_KEPT {
            self.joined.remove(0);
        }
    }

    /// Join unticked. Returns whether it was ticked.
    pub fn untick(&mut self, raid_id: &str) -> bool {
        let n = self.joined.len();
        self.joined.retain(|(id, _)| id != raid_id);
        self.joined.len() != n
    }

    /// The raid ended: its tick goes, and tells who ticked it.
    pub fn take_tick(&mut self, raid_id: &str) -> Option<String> {
        let i = self.joined.iter().position(|(id, _)| id == raid_id)?;
        Some(self.joined.remove(i).1)
    }

    /// V110-F72 A ticked raid went: kept until the page acknowledges it.
    pub fn keep_go(&mut self, raid: RaidStatus, account: String, now: i64) {
        self.pending.retain(|(_, r, _)| r.id != raid.id);
        self.pending.push((now, raid, account));
    }

    /// V110-F72 The page opened (or means to open) the raid: it is not kept any more.
    pub fn ack_go(&mut self, raid_id: &str) {
        self.pending.retain(|(_, r, _)| r.id != raid_id);
    }

    /// V110-F72 What a page that just loaded has missed (acknowledged by reading); old ones drop.
    pub fn take_pending(&mut self, now: i64) -> Vec<(RaidStatus, String)> {
        let kept = std::mem::take(&mut self.pending);
        kept.into_iter().filter(|(at, ..)| at + PENDING_SECS > now).map(|(_, r, a)| (r, a)).collect()
    }

    pub fn ticked(&self) -> Vec<String> {
        self.joined.iter().map(|(id, _)| id.clone()).collect()
    }

    pub fn forget(&mut self, channel_id: &str) {
        self.by_channel.remove(channel_id);
    }
}

static RAIDS: LazyLock<Mutex<Raids>> = LazyLock::new(Mutex::default);

fn raids() -> std::sync::MutexGuard<'static, Raids> {
    RAIDS.lock().unwrap_or_else(|p| p.into_inner())
}

/// The chat line when it goes: "Raided Kuni with 1.234 viewers."
pub fn went_line(r: &RaidStatus) -> String {
    let n = r.viewers;
    format!("Raided {} with {} {}.", r.target_name, grouped(n), if n == 1 { "viewer" } else { "viewers" })
}

/// V110-N11 The chat line when a raid starts.
pub fn started_line(r: &RaidStatus) -> String {
    format!("Raiding {}.", r.target_name)
}

/// V110-N11 The chat line when a raid is called off.
pub fn canceled_line() -> &'static str {
    "Raid canceled."
}

/// V110-N11 How old a raid may be and still count as just started: a chat that opens mid-raid
/// (or a topic that was forgotten) sees only updates, which must not read as a new raid.
const FRESH_SECS: i64 = 10;

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
    let done = match &ev {
        RaidEvent::Done { id } => Some(id.clone()),
        RaidEvent::Update(_) => None,
    };
    let (step, started) = {
        let mut r = raids();
        let now = crate::auth::unix_now();
        r.purge(now);
        let started = r.starts(id, &ev, now);
        (r.step(id, ev, went), started)
    };
    let (raid, went) = match step {
        Step::Keep => {
            // A go or cancel with no banner for it still takes the tick.
            if let Some(raid_id) = done
                && raids().take_tick(&raid_id).is_some()
            {
                let _ = app.emit(JOIN_EVENT, RaidJoinChanged { raid_id, on: false });
            }
            return;
        }
        Step::Show(r) => {
            if started && let Some(login) = app.state::<Chat>().channels.login_of(id) {
                super::notice_line(app, &login, NoticeKind::Raid, &started_line(&r));
            }
            (Some(r), false)
        }
        Step::Gone(r) => {
            if let Some(login) = app.state::<Chat>().channels.login_of(id) {
                super::notice_line(app, &login, NoticeKind::Raid, canceled_line());
            }
            // Called off: nothing opens and the tick clears.
            let ticked = raids().take_tick(&r.id).is_some();
            if ticked {
                let _ = app.emit(JOIN_EVENT, RaidJoinChanged { raid_id: r.id, on: false });
            }
            (None, false)
        }
        Step::Went(r) => {
            if let Some(login) = app.state::<Chat>().channels.login_of(id) {
                super::notice_line(app, &login, NoticeKind::Raid, &went_line(&r));
            }
            // Only a ticked raid opens its channel, once.
            let who = raids().take_tick(&r.id);
            if let Some(account) = who {
                let _ = app.emit(JOIN_EVENT, RaidJoinChanged { raid_id: r.id.clone(), on: false });
                go(app, RaidGo { raid: r, account });
            }
            (None, true)
        }
    };
    let _ = app.emit(EVENT, RaidChanged { channel_id: id.clone(), raid, went });
}

/// V110-F72 The pop-out is asked for: kept until the main page acknowledges it, in case it
/// is reloading and misses the event.
fn go(app: &AppHandle, g: RaidGo) {
    raids().keep_go(g.raid.clone(), g.account.clone(), crate::auth::unix_now());
    let _ = app.emit_to(crate::windows::MAIN, GO_EVENT, g);
}

/// V110-F72 Gone raids the main page has not opened yet; reading takes them.
pub fn pending_go() -> Vec<RaidGo> {
    let list = raids().take_pending(crate::auth::unix_now());
    list.into_iter().map(|(raid, account)| RaidGo { raid, account }).collect()
}

/// V110-F72 The page opens this raid (from the live event): not kept any more.
pub fn ack_go(raid_id: &str) {
    raids().ack_go(raid_id);
}

/// Twitch, or in test builds the raid mock when it is set.
fn http(app: &AppHandle) -> cyex_twitch::Http {
    let http = app.state::<crate::auth::Auth>().http.clone();
    #[cfg(feature = "mocks")]
    let http = super::raid_mock::http(app, http);
    http
}

/// The raids with Join ticked, for a chat that opens mid-raid.
pub fn ticked() -> Vec<String> {
    raids().ticked()
}

/// Join unticked before the raid went: only the pop-up is dropped (no leave call is known).
pub fn leave(app: &AppHandle, raid_id: &str) {
    let was = raids().untick(raid_id);
    if was {
        let _ = app.emit(JOIN_EVENT, RaidJoinChanged { raid_id: raid_id.to_owned(), on: false });
    }
}

/// Joins the raid as `account`, only ever from the banner's tick. Ticked on success.
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
    })?;
    // The raid may have gone while Twitch took the join: tick only if it didn't.
    let tick = raids().tick_live(raid_id, account);
    match tick {
        Tick::On => {
            let _ = app.emit(JOIN_EVENT, RaidJoinChanged { raid_id: raid_id.to_owned(), on: true });
            Ok(())
        }
        // Twitch joined us and the raid went: the pop-out opens as if the tick had been there.
        Tick::Ended(Some(raid)) => {
            let account = account.to_owned();
            go(app, RaidGo { raid, account });
            Ok(())
        }
        Tick::Ended(None) => Err("The raid already ended.".to_owned()),
    }
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
        assert_eq!(r.step("7", RaidEvent::Done { id: "a".into() }, false), Step::Gone(raid("a", 5)));
        assert_eq!(r.current("8", 1000), Some(raid("b", 6)));
        // A new raid after a cancel shows.
        assert_eq!(r.step("7", RaidEvent::Update(raid("c", 1)), false), Step::Show(raid("c", 1)));
    }

    #[test]
    fn tick_is_per_raid_and_opens_only_on_go() {
        let mut r = Raids::default();
        r.step("7", RaidEvent::Update(raid("a", 5)), false);
        assert_eq!(r.take_tick("a"), None, "go without a tick opens nothing");
        r.tick("a", "900");
        assert_eq!(r.ticked(), vec!["a".to_owned()]);
        // A new raid starts unticked.
        r.step("7", RaidEvent::Done { id: "a".into() }, false);
        r.step("7", RaidEvent::Update(raid("b", 5)), false);
        assert!(!r.ticked().contains(&"b".to_owned()));
        // The tick goes with the raid: taken once, by who ticked it.
        assert_eq!(r.take_tick("a"), Some("900".to_owned()));
        assert_eq!(r.take_tick("a"), None);
    }

    #[test]
    fn untick_drops_the_tick_and_cancel_clears_it() {
        let mut r = Raids::default();
        r.tick("a", "900");
        assert!(r.untick("a"));
        assert!(!r.untick("a"));
        assert_eq!(r.take_tick("a"), None, "unticked: go opens nothing");
        r.tick("c", "900");
        r.step("7", RaidEvent::Update(raid("c", 1)), false);
        assert_eq!(r.step("7", RaidEvent::Done { id: "c".into() }, false), Step::Gone(raid("c", 1)));
        assert_eq!(r.take_tick("c"), Some("900".to_owned()), "taken by the cancel handler, then gone");
        assert_eq!(r.take_tick("c"), None);
    }

    #[test]
    fn a_join_that_finishes_after_the_go_is_not_ticked() {
        let mut r = Raids::default();
        r.step("7", RaidEvent::Update(raid("a", 5)), false);
        assert_eq!(r.tick_live("a", "900"), Tick::On);
        r.untick("a");
        r.step("7", RaidEvent::Done { id: "a".into() }, true);
        // The go came first: nothing stale is left, and the raid comes back to open.
        assert_eq!(r.tick_live("a", "900"), Tick::Ended(Some(raid("a", 5))));
        assert!(r.ticked().is_empty());
        // Called off: no raid to open.
        r.step("7", RaidEvent::Update(raid("b", 5)), false);
        r.step("7", RaidEvent::Done { id: "b".into() }, false);
        assert_eq!(r.tick_live("b", "900"), Tick::Ended(None));
        assert!(r.ticked().is_empty());
    }

    #[test]
    fn a_go_without_a_banner_still_ends_the_raid() {
        let mut r = Raids::default();
        r.tick("x", "900");
        assert_eq!(r.step("7", RaidEvent::Done { id: "x".into() }, true), Step::Keep);
        // The handler takes the tick on a Keep.
        assert_eq!(r.take_tick("x"), Some("900".to_owned()));
        assert_eq!(r.tick_live("x", "900"), Tick::Ended(None));
    }

    #[test]
    fn v110_f72_a_missed_go_waits_to_be_read_once() {
        let mut r = Raids::default();
        r.keep_go(raid("a", 5), "900".into(), 1000);
        r.keep_go(raid("b", 7), "901".into(), 1010);
        // The same raid twice is kept once.
        r.keep_go(raid("b", 8), "901".into(), 1011);
        let got = r.take_pending(1020);
        assert_eq!(got.iter().map(|(x, _)| x.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(got[1].0.viewers, 8);
        assert_eq!(got[0].1, "900");
        assert!(r.take_pending(1021).is_empty(), "reading takes them");
    }

    #[test]
    fn v110_f72_an_acknowledged_or_old_go_is_not_offered() {
        let mut r = Raids::default();
        r.keep_go(raid("a", 5), "900".into(), 1000);
        r.keep_go(raid("b", 5), "900".into(), 1000);
        r.ack_go("a");
        let got = r.take_pending(1000 + PENDING_SECS - 1);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0.id, "b");
        r.keep_go(raid("c", 5), "900".into(), 1000);
        assert!(r.take_pending(1000 + PENDING_SECS).is_empty(), "past 60 s it is dropped");
    }

    #[test]
    fn purge_drops_old_banner_state() {
        let mut r = Raids::default();
        r.step("7", RaidEvent::Update(raid("a", 5)), false);
        r.purge(1090 + STALE_SECS - 1);
        assert!(r.by_channel.contains_key("7"));
        r.purge(1090 + STALE_SECS);
        assert!(r.by_channel.is_empty());
    }

    #[test]
    fn ticks_are_bounded() {
        let mut r = Raids::default();
        for i in 0..(JOINED_KEPT + 5) {
            r.tick(&format!("r{i}"), "900");
        }
        assert_eq!(r.ticked().len(), JOINED_KEPT);
        assert!(!r.ticked().contains(&"r0".to_owned()));
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
        assert_eq!(went_line(&raid("a", 1234)), "Raided Kuni with 1.234 viewers.");
        assert_eq!(went_line(&raid("a", 1)), "Raided Kuni with 1 viewer.");
    }

    #[test]
    fn v110_n11_the_start_line_is_due_once_and_only_for_a_new_raid() {
        let mut r = Raids::default();
        let a = raid("a", 5);
        let ev = RaidEvent::Update(a.clone());
        let now = a.created_at + 1;
        assert!(r.starts("c", &ev, now));
        r.step("c", ev.clone(), false);
        assert!(!r.starts("c", &RaidEvent::Update(raid("a", 9)), now), "an update of the same raid");
        assert!(r.starts("c", &RaidEvent::Update(raid("b", 5)), now), "another raid");
        assert!(!r.starts("c", &ev, a.created_at + 60), "a chat opened mid-raid");
        r.step("c", RaidEvent::Done { id: "a".into() }, true);
        assert!(!r.starts("c", &ev, now), "an ended raid");
        assert_eq!(started_line(&a), "Raiding Kuni.");
        assert_eq!(canceled_line(), "Raid canceled.");
    }
}
