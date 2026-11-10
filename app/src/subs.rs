//! 1.28 Subscriptions without twitch.tv: the account's subs, its standing in a channel,
//! sharing a resub in chat, the free Prime sub and turning renewal off, all through GQL.
//! Buying, gifting, changing tier and payment methods open Twitch's own pages in the
//! app's Twitch window (`windows::twitch`, 3.8).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use cyex_miner::events::{self as miner_events, Event as MinerEvent};
use cyex_twitch::gql::milestones::{self, Milestone, Read};
use cyex_twitch::gql::subs::{self as api, ChannelSub, Subs, pages};
use cyex_twitch::hermes::{Event, Kind};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use crate::whispers::GqlCall;

const NO_GQL: &str = "This needs a GQL sign-in for the account (Settings → Accounts).";

fn call(app: &AppHandle, account: &str) -> Result<GqlCall, String> {
    GqlCall::new(app, account).map_err(|_| NO_GQL.to_owned())
}

fn failed(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        return "Couldn’t reach Twitch.".into();
    }
    eprintln!("subs: {e}");
    if e.is_integrity() {
        "Twitch wants this done on its own page.".into()
    } else {
        "Twitch didn’t take that. Try again in a bit.".into()
    }
}

pub async fn mine(app: &AppHandle, account: &str) -> Result<Subs, String> {
    let c = call(app, account)?;
    api::mine(&c.http, c.auth()).await.map_err(failed)
}

pub async fn channel(app: &AppHandle, account: &str, login: &str) -> Result<Option<ChannelSub>, String> {
    let c = call(app, account)?;
    api::channel(&c.http, c.auth(), login).await.map_err(failed)
}

pub async fn share_resub(
    app: &AppHandle,
    account: &str,
    login: &str,
    token_id: &str,
    message: &str,
    streak: bool,
) -> Result<(), String> {
    if message.chars().count() > 500 {
        return Err("The message can be 500 characters at most.".into());
    }
    let c = call(app, account)?;
    api::share_resub(&c.http, c.auth(), login, token_id, message.trim(), streak)
        .await
        .map_err(failed)
}

pub async fn prime(app: &AppHandle, account: &str, channel_id: &str) -> Result<(), String> {
    let c = call(app, account)?;
    api::prime(&c.http, c.auth(), account, channel_id)
        .await
        .map_err(|e| match e.api_message() {
            Some("NO_CREDIT" | "NO_SUB_CREDIT") => "Your Prime sub is already in use this month.".into(),
            Some("ALREADY_SUBSCRIBED" | "EXISTING_SUBSCRIPTION") => "You’re already subscribed there.".into(),
            _ => failed(e),
        })
}

pub async fn stop_renewal(app: &AppHandle, account: &str, sub_id: &str) -> Result<(), String> {
    let c = call(app, account)?;
    api::stop_renewal(&c.http, c.auth(), account, sub_id)
        .await
        .map_err(failed)
}

/// V108-N23 Tells open chats to read the watch streak again.
const STREAK_EVENT: &str = "subs:streak";

/// V108-N23 What the account can share in a channel now, and whether the channel is live (the
/// chat then re-reads every minute as a fallback to the PubSub topic). `channel_id` is the
/// channel Twitch answered for, so a chat drops an answer meant for another one.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Streak {
    pub milestone: Option<Milestone>,
    pub live: bool,
    pub channel_id: Option<String>,
}

/// V108-N23 A milestone or streak changed in this channel.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreakChanged {
    pub account: String,
    pub channel_id: String,
}

/// V108-N23 A streak read is kept this long per (account, channel login), so every chat of a
/// channel and each reopen don't each ask Twitch. A change on the topic drops that channel's.
const STREAK_AGE: Duration = Duration::from_secs(60);
/// The most reads kept (channels and ids four times that); past it the oldest go.
const STREAK_KEPT: usize = 64;

/// What the app knows about one account's streak in one channel.
#[derive(Clone, Copy, Debug, Default)]
struct Seen {
    /// The count shared here last, and when.
    shared: Option<(u32, Instant)>,
    /// The last "streak reached" event: its count (when it said one) and when.
    reached: Option<(Option<u32>, Instant)>,
}

impl Seen {
    fn last(&self) -> Option<Instant> {
        self.shared.map(|(_, t)| t).max(self.reached.map(|(_, t)| t))
    }
}

/// Makes room for one more under `cap` by dropping the oldest entries (by `age`).
fn make_room<K: Clone + Eq + std::hash::Hash, V, A: Ord>(map: &mut HashMap<K, V>, cap: usize, age: impl Fn(&V) -> A) {
    while map.len() >= cap {
        let Some(k) = map.iter().min_by_key(|(_, v)| age(v)).map(|(k, _)| k.clone()) else { return };
        map.remove(&k);
    }
}

/// V108-N23 Reads kept a minute, plus what decides whether a read's streak is offered: ids
/// shared or refused never come back, a share hides the channel's row until a later "reached"
/// event and a higher count, and a streak above that event's count is never offered (user
/// report 2026/11/01: a 3 shared, then a 5 showed that Twitch wouldn't share).
#[derive(Default)]
struct StreakBook {
    reads: HashMap<(String, String), (Instant, Read)>,
    seen: HashMap<(String, String), Seen>,
    /// Ids shared or refused, and when.
    gone: HashMap<String, Instant>,
    /// Per account, bumped by every "reached" event or share: a read asked for before one
    /// is not kept (it may predate the change).
    generation: HashMap<String, u64>,
}

impl StreakBook {
    fn read(&self, key: &(String, String), now: Instant) -> Option<Read> {
        self.reads.get(key).filter(|(at, _)| now.duration_since(*at) < STREAK_AGE).map(|(_, r)| r.clone())
    }

    fn generation(&self, account: &str) -> u64 {
        self.generation.get(account).copied().unwrap_or(0)
    }

    /// Keeps a read asked for at `generation` (see [`Self::generation`]), unless a change came since.
    fn put(&mut self, key: (String, String), read: Read, now: Instant, generation: u64) {
        if self.generation(&key.0) != generation {
            return;
        }
        self.reads.retain(|_, (at, _)| now.duration_since(*at) < STREAK_AGE);
        if !self.reads.contains_key(&key) {
            make_room(&mut self.reads, STREAK_KEPT, |(at, _)| *at);
        }
        self.reads.insert(key, (now, read));
    }

    /// Drops the account's read of this channel; reads asked for before now aren't kept.
    fn forget(&mut self, account: &str, channel_id: &str) {
        *self.generation.entry(account.to_owned()).or_default() += 1;
        self.reads.retain(|(a, _), (_, r)| a != account || r.channel_id.as_deref() != Some(channel_id));
    }

    fn seen(&mut self, account: &str, channel_id: &str) -> &mut Seen {
        let key = (account.to_owned(), channel_id.to_owned());
        if !self.seen.contains_key(&key) {
            make_room(&mut self.seen, STREAK_KEPT * 4, Seen::last);
        }
        self.seen.entry(key).or_default()
    }

    /// The streak to offer from a read, or none.
    fn offer(&self, account: &str, read: &Read) -> Option<Milestone> {
        let seen = read
            .channel_id
            .as_ref()
            .and_then(|c| self.seen.get(&(account.to_owned(), c.clone())))
            .copied()
            .unwrap_or_default();
        let ceiling = seen.reached.and_then(|(v, _)| v);
        let m = milestones::pick(&read.milestones, ceiling, |id| self.gone.contains_key(id))?;
        if let Some((value, at)) = seen.shared {
            // After a share only a newer "reached" event and a higher count bring it back.
            let newer = seen.reached.is_some_and(|(_, r)| r > at);
            if !newer || m.value <= value {
                return None;
            }
        }
        Some(m.clone())
    }

    /// A "reached" event (topic or points) for this channel. One without a count (a points
    /// gain) keeps the last count as the ceiling.
    fn reached(&mut self, account: &str, channel_id: &str, value: Option<u32>, now: Instant) {
        self.forget(account, channel_id);
        let seen = self.seen(account, channel_id);
        let value = value.or_else(|| seen.reached.and_then(|(v, _)| v));
        seen.reached = Some((value, now));
    }

    /// A share of the streak with this count went through: the id is gone and the channel
    /// waits for a new event.
    fn shared(&mut self, account: &str, channel_id: &str, milestone_id: &str, value: u32, now: Instant) {
        self.refused(milestone_id, now);
        self.forget(account, channel_id);
        self.seen(account, channel_id).shared = Some((value, now));
    }

    /// Shared, or Twitch wouldn't share it: never offered again this run.
    fn refused(&mut self, milestone_id: &str, now: Instant) {
        if !self.gone.contains_key(milestone_id) {
            make_room(&mut self.gone, STREAK_KEPT * 4, |at| *at);
        }
        self.gone.insert(milestone_id.to_owned(), now);
    }
}

static STREAKS: LazyLock<Mutex<StreakBook>> = LazyLock::new(Mutex::default);

fn streaks() -> std::sync::MutexGuard<'static, StreakBook> {
    STREAKS.lock().unwrap_or_else(|p| p.into_inner())
}

pub async fn streak(app: &AppHandle, account: &str, login: &str) -> Result<Streak, String> {
    let key = (account.to_owned(), login.to_ascii_lowercase());
    let live = crate::tabs::live::is_live(app, login);
    let (cached, generation) = {
        let book = streaks();
        (book.read(&key, Instant::now()), book.generation(account))
    };
    let read = match cached {
        Some(r) => r,
        None => {
            let c = call(app, account)?;
            let r = milestones::milestones(&c.http, c.auth(), login).await.map_err(failed)?;
            streaks().put(key, r.clone(), Instant::now(), generation);
            r
        }
    };
    let milestone = streaks().offer(account, &read);
    Ok(Streak { milestone, live, channel_id: read.channel_id })
}

/// V108-N23 Only from the user's click on Share. `value` is the shown streak's count.
pub async fn share_streak(app: &AppHandle, account: &str, channel_id: &str, milestone_id: &str, value: u32) -> Result<(), String> {
    let c = call(app, account)?;
    let out = milestones::share(&c.http, c.auth(), channel_id, milestone_id).await;
    let mut book = streaks();
    match out {
        Ok(()) => {
            book.shared(account, channel_id, milestone_id, value, Instant::now());
            Ok(())
        }
        Err(cyex_twitch::Error::Refused(code)) => {
            book.refused(milestone_id, Instant::now());
            book.forget(account, channel_id);
            Err(milestones::explain(&code).to_owned())
        }
        Err(e) => Err(failed(e)),
    }
}

/// V108-N23 Follows (or stops following) the account's milestones topic while a chat shows
/// the prompt. The topic is counted, so several chats can share it; held per window
/// (`label`), so a page that goes away without its cleanup lets go too.
pub fn watch_streak(app: &AppHandle, label: &str, account: &str, on: bool) {
    crate::hermes::hold(app, label, crate::hermes::Held::User(account.to_owned(), Kind::ViewerMilestones), on);
}

/// V108-N23 A streak reached on the milestones topic: the chat of that channel reads again.
pub fn on_milestones(app: &AppHandle, e: &Event) {
    if let Some((channel_id, value)) = milestones::update_of(&e.payload) {
        changed(app, &e.topic.channel_id, channel_id, value);
    }
}

/// V108-N23 A `WATCH_STREAK` points gain on the account's points topic: the streak moved.
pub fn on_points(app: &AppHandle, e: &Event) {
    if let Some(MinerEvent::Earned { channel_id, reason, .. }) = miner_events::parse(&e.payload)
        && reason == milestones::WATCH_STREAK
    {
        changed(app, &e.topic.channel_id, channel_id, None);
    }
}

fn changed(app: &AppHandle, account: &str, channel_id: String, value: Option<u32>) {
    // The read again must reach Twitch (the cache is by login, the topic says the id).
    streaks().reached(account, &channel_id, value, Instant::now());
    let _ = app.emit(STREAK_EVENT, StreakChanged { account: account.to_owned(), channel_id });
}

/// Which Twitch page a sub action opens in the app's Twitch window.
pub fn page(kind: &str, login: &str) -> Option<String> {
    let login_ok = cyex_core::login::is_login(login);
    if matches!(kind, "subscribe" | "gift") && !login_ok {
        return None;
    }
    Some(match kind {
        "wallet" => pages::WALLET.to_owned(),
        "subscriptions" => pages::SUBSCRIPTIONS.to_owned(),
        "bits" => pages::BITS.to_owned(),
        "subscribe" => pages::subscribe(login),
        "gift" => pages::gift(login),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_page_needs_no_channel() {
        assert_eq!(page("bits", "").as_deref(), Some("https://www.twitch.tv/bits"));
        assert_eq!(page("bits", "x").as_deref(), Some(pages::BITS));
        assert_eq!(page("nope", ""), None);
        assert_eq!(page("gift", ""), None);
    }

    #[test]
    fn streak_reads_kept_a_minute_and_bounded() {
        let mut c = StreakBook::default();
        let t = Instant::now();
        let key = |a: &str, l: &str| (a.to_owned(), l.to_owned());
        c.put(key("1", "x"), Read::default(), t, 0);
        assert_eq!(c.read(&key("1", "x"), t + Duration::from_secs(59)), Some(Read::default()));
        assert_eq!(c.read(&key("1", "x"), t + STREAK_AGE), None);
        // Expired reads go on the next put.
        c.put(key("1", "y"), Read::default(), t + STREAK_AGE, 0);
        assert_eq!(c.reads.len(), 1);
        // Past the bound the oldest go, not all.
        let at = |i: usize| t + STREAK_AGE + Duration::from_millis(i as u64);
        for i in 0..STREAK_KEPT * 2 {
            c.put(key("2", &i.to_string()), read(&i.to_string(), &[]), at(i), 0);
        }
        assert_eq!(c.reads.len(), STREAK_KEPT);
        assert!(c.reads.contains_key(&key("2", &(STREAK_KEPT * 2 - 1).to_string())));
        assert!(!c.reads.contains_key(&key("2", "0")));
        // A change in one channel drops only that channel's read.
        c.forget("2", &(STREAK_KEPT * 2 - 1).to_string());
        assert_eq!(c.reads.len(), STREAK_KEPT - 1);
    }

    /// A read asked for before a "reached" event or a share answers but isn't kept.
    #[test]
    fn a_read_from_before_a_change_is_not_kept() {
        let mut b = StreakBook::default();
        let t = Instant::now();
        let key = ("1".to_owned(), "chan".to_owned());
        let generation = b.generation("1");
        b.reached("1", "9", Some(3), t);
        b.put(key.clone(), read("9", &[m("a3", 3, "CAN_SHARE")]), t, generation);
        assert_eq!(b.read(&key, t), None);
        // Asked for after it: kept.
        let generation = b.generation("1");
        b.put(key.clone(), read("9", &[m("a3", 3, "CAN_SHARE")]), t, generation);
        assert!(b.read(&key, t).is_some());
        // Another channel's event leaves it.
        b.reached("1", "8", Some(1), t);
        assert!(b.read(&key, t).is_some());
    }

    #[test]
    fn a_reached_event_without_a_count_keeps_the_ceiling() {
        let mut b = StreakBook::default();
        let t = Instant::now();
        let r = read("9", &[m("a5", 5, "CAN_SHARE")]);
        b.reached("1", "9", Some(4), t);
        assert_eq!(b.offer("1", &r), None);
        b.reached("1", "9", None, t + Duration::from_secs(1));
        assert_eq!(b.offer("1", &r), None);
        b.reached("1", "9", Some(5), t + Duration::from_secs(2));
        assert_eq!(value(b.offer("1", &r)), Some(5));
    }

    /// The shared count comes from the caller, not from a read that may be gone by then.
    #[test]
    fn a_share_keeps_its_count_without_a_read() {
        let mut b = StreakBook::default();
        let t = Instant::now();
        b.shared("1", "9", "a3", 3, t);
        b.reached("1", "9", Some(5), t + Duration::from_secs(1));
        assert_eq!(b.offer("1", &read("9", &[m("b3", 3, "CAN_SHARE")])), None);
        assert_eq!(value(b.offer("1", &read("9", &[m("a5", 5, "CAN_SHARE")]))), Some(5));
    }

    #[test]
    fn past_the_bound_the_newest_ids_and_channels_stay() {
        let mut b = StreakBook::default();
        let t = Instant::now();
        let at = |i: usize| t + Duration::from_millis(i as u64);
        for i in 0..STREAK_KEPT * 4 + 10 {
            b.refused(&i.to_string(), at(i));
            b.reached("1", &i.to_string(), Some(1), at(i));
        }
        assert_eq!(b.gone.len(), STREAK_KEPT * 4);
        assert_eq!(b.seen.len(), STREAK_KEPT * 4);
        let last = (STREAK_KEPT * 4 + 9).to_string();
        assert!(b.gone.contains_key(&last) && !b.gone.contains_key("0"));
        assert!(b.seen.contains_key(&("1".to_owned(), last)));
    }

    fn m(id: &str, value: u32, share: &str) -> Milestone {
        Milestone {
            id: id.into(),
            category: milestones::WATCH_STREAK.into(),
            value,
            state: "ACTIVE".into(),
            share_status: share.into(),
        }
    }

    fn read(channel: &str, ms: &[Milestone]) -> Read {
        Read { channel_id: Some(channel.into()), milestones: ms.to_vec() }
    }

    fn value(o: Option<Milestone>) -> Option<u32> {
        o.map(|m| m.value)
    }

    /// User report 2026/11/01: shared a 3, then a 5 showed that Twitch wouldn't share.
    #[test]
    fn after_a_share_only_a_newer_reached_streak_shows() {
        let mut b = StreakBook::default();
        let t = Instant::now();
        let s = |n: u64| t + Duration::from_secs(n);
        let before = read("9", &[m("a3", 3, "CAN_SHARE"), m("a5", 5, "CAN_SHARE")]);
        // The smaller one is offered, not the next threshold.
        assert_eq!(value(b.offer("1", &before)), Some(3));
        b.put(("1".into(), "chan".into()), before, s(0), 0);
        b.shared("1", "9", "a3", 3, s(1));
        assert!(b.reads.is_empty());
        // The read right after (the share's own update, a poll) still lists a 5: hidden.
        let after = read("9", &[m("a3", 3, "SHARED"), m("a5", 5, "CAN_SHARE")]);
        assert_eq!(b.offer("1", &after), None);
        // A "reached" event says 5, but the read doesn't confirm a shareable one: hidden.
        b.reached("1", "9", Some(5), s(2));
        assert_eq!(b.offer("1", &read("9", &[m("a3", 3, "SHARED")])), None);
        assert_eq!(b.offer("1", &read("9", &[m("a5", 5, "CAN_NOT_SHARE")])), None);
        // The read confirms it: shown.
        assert_eq!(value(b.offer("1", &after)), Some(5));
        // An event that says 4 is the ceiling: a 5 is not real yet.
        b.reached("1", "9", Some(4), s(3));
        assert_eq!(b.offer("1", &after), None);
    }

    #[test]
    fn streaks_stay_with_their_channel_and_account() {
        let mut b = StreakBook::default();
        let t = Instant::now();
        b.shared("1", "9", "x", 4, t);
        // Another channel, or another account in this one, isn't held back by that share.
        assert_eq!(value(b.offer("1", &read("8", &[m("y", 5, "CAN_SHARE")]))), Some(5));
        assert_eq!(value(b.offer("2", &read("9", &[m("z", 5, "CAN_SHARE")]))), Some(5));
        // A ceiling from one channel doesn't cap another.
        b.reached("1", "7", Some(2), t);
        assert_eq!(value(b.offer("1", &read("8", &[m("y", 5, "CAN_SHARE")]))), Some(5));
    }

    #[test]
    fn a_refused_streak_never_comes_back() {
        let mut b = StreakBook::default();
        let r = read("9", &[m("a", 3, "CAN_SHARE")]);
        assert!(b.offer("1", &r).is_some());
        b.refused("a", Instant::now());
        assert_eq!(b.offer("1", &r), None);
    }
}
