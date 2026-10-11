//! One joined channel (1.8): message buffer, room state, our sends and their echoes,
//! deletions, history (C1), the user's rules (`checks`). Pure state: the app feeds it
//! lines and streams the events.

mod checks;
mod gap;
mod hook;
mod presence;
mod role;

pub use hook::{LineChange, LineHook};

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use serde::Serialize;
use ts_rs::TS;

use crate::build::{self, Colors, Ctx, GiftLink, Incoming, Me, UserState};
use crate::irc::Line;
use crate::rules::{Outcome, Rules};
use crate::similar::Repeats;
use crate::model::{
    ChatMessage, ConnStatus, Flag, Meta, NoticeKind, Pinned, Reply, RoomEvent, RoomState, Sender, plain,
};
use crate::tokenize::{Emotes, Input, tokenize};

/// Fewest messages kept per room, for new views and usercards. The scrollback setting
/// can raise it (V101-02).
pub const BUFFER: usize = 500;
/// A send without an answer from Twitch after this long is forgotten.
const PENDING_TTL_MS: i64 = 15_000;
/// C16 Gift subs arrive right after their mass gift; later ones stand alone.
const GIFT_MERGE_MS: i64 = 30_000;

/// What a chat view needs when it opens.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomSnapshot {
    pub login: String,
    #[ts(as = "Vec<ChatMessage>")]
    pub messages: Vec<Arc<ChatMessage>>,
    pub state: RoomState,
    pub status: ConnStatus,
    pub pinned: Option<Pinned>,
}

/// A message we sent that Twitch hasn't confirmed yet.
#[derive(Clone, Debug)]
pub struct PendingSend {
    pub text: String,
    pub action: bool,
    pub reply: Option<Reply>,
    pub at_ms: i64,
}

/// C16 A mass gift waiting for its single gifts.
struct GiftGroup {
    origin: String,
    message_id: String,
    size: usize,
    expires_ms: i64,
}

pub struct Room {
    pub login: String,
    pub state: RoomState,
    pub status: ConnStatus,
    me: Option<Me>,
    buffer: VecDeque<Arc<ChatMessage>>,
    /// V101-02 Messages kept, at least `BUFFER`.
    keep: usize,
    pending: VecDeque<PendingSend>,
    gifts: Vec<GiftGroup>,
    /// 1.9 Reward titles and costs by id, to name redemptions.
    rewards: HashMap<String, (String, u32)>,
    pinned: Option<Pinned>,
    colors: Colors,
    rules: Arc<Rules>,
    /// S5 Plugins' look at each line.
    hook: Option<Arc<dyn LineHook>>,
    repeats: Repeats,
    /// What live messages should make the app do, until it takes them.
    outcomes: Vec<(Arc<ChatMessage>, Outcome)>,
    /// C24 When the connection dropped, while it's gone.
    lost_at: Option<i64>,
    /// C37, V102-18 The joins and leaves lines still taking names.
    presence: presence::Batches,
}

impl Room {
    pub fn new(login: &str, me: Option<Me>) -> Self {
        let mut room = Self {
            login: login.to_ascii_lowercase(),
            state: RoomState::default(),
            status: ConnStatus::Connecting,
            me,
            buffer: VecDeque::new(),
            keep: BUFFER,
            pending: VecDeque::new(),
            gifts: Vec::new(),
            rewards: HashMap::new(),
            pinned: None,
            colors: Colors::default(),
            rules: Arc::default(),
            hook: None,
            repeats: Repeats::default(),
            outcomes: Vec::new(),
            lost_at: None,
            presence: presence::Batches::default(),
        };
        room.state.own = room.initial_own();
        room
    }

    /// S5 `None` when no plugin reads chat.
    pub fn set_hook(&mut self, hook: Option<Arc<dyn LineHook>>) {
        self.hook = hook;
    }

    /// A message still in the buffer.
    pub fn message(&self, id: &str) -> Option<Arc<ChatMessage>> {
        self.buffer.iter().rev().find(|m| m.id == id).cloned()
    }

    pub fn me(&self) -> Option<&Me> {
        self.me.as_ref()
    }

    pub fn snapshot(&self) -> RoomSnapshot {
        self.snapshot_tail(self.keep)
    }

    /// V101-02 Messages kept from now on; never fewer than `BUFFER`.
    pub fn set_keep(&mut self, lines: usize) {
        self.keep = lines.max(BUFFER);
        self.trim();
    }

    pub fn keep(&self) -> usize {
        self.keep
    }

    fn trim(&mut self) {
        let over = self.buffer.len().saturating_sub(self.keep);
        self.buffer.drain(..over);
    }

    /// Like `snapshot`, with only the last `lines` messages.
    pub fn snapshot_tail(&self, lines: usize) -> RoomSnapshot {
        let skip = self.buffer.len().saturating_sub(lines);
        RoomSnapshot {
            login: self.login.clone(),
            messages: self.buffer.iter().skip(skip).cloned().collect(),
            state: self.state.clone(),
            status: self.status,
            pinned: self.pinned.clone(),
        }
    }

    pub fn colors(&mut self) -> &mut Colors {
        &mut self.colors
    }

    /// 1.9 Names future redemptions from the channel's reward list.
    pub fn set_rewards(&mut self, rewards: HashMap<String, (String, u32)>) {
        self.rewards = rewards;
    }

    /// Returns an event if the pin changed.
    pub fn set_pinned(&mut self, pin: Option<Pinned>) -> Option<RoomEvent> {
        (self.pinned != pin).then(|| {
            self.pinned = pin.clone();
            RoomEvent::Pinned { pin: pin.map(Box::new) }
        })
    }

    /// The reply header for answering message `id`, if it's still here.
    pub fn reply_to(&self, id: &str) -> Option<Reply> {
        let m = self.buffer.iter().find(|m| m.id == id)?;
        let s = m.sender.as_ref()?;
        Some(Reply {
            parent_id: m.id.clone(),
            parent_login: s.login.clone(),
            parent_display: s.display.clone(),
            parent_body: plain(&m.body),
            thread_id: m.reply.as_ref().map_or_else(|| m.id.clone(), |r| r.thread_id.clone()),
        })
    }

    /// Broadcaster, lead moderator or moderator here (D7, slow mode).
    pub fn privileged(&self) -> bool {
        self.role().is_some()
    }

    /// The last `n` messages from a user, oldest first (usercard, 1.36).
    pub fn recent_from(&self, user_id: &str, n: usize) -> Vec<Arc<ChatMessage>> {
        let mut out: Vec<_> = self
            .buffer
            .iter()
            .rev()
            .filter(|m| m.notice.is_none() && m.sender_id() == Some(user_id))
            .take(n)
            .cloned()
            .collect();
        out.reverse();
        out
    }

    /// C9 A sender's emotes arrived: their messages since `since_ms` are read again.
    pub fn retokenize_from(&mut self, user_id: &str, since_ms: i64, emotes: &dyn Emotes) -> Vec<RoomEvent> {
        let mut events = Vec::new();
        for slot in self.buffer.iter_mut().rev() {
            if slot.ts < since_ms {
                break;
            }
            if slot.sender_id() != Some(user_id) {
                continue;
            }
            if let Some(body) = crate::tokenize::retokenize(&slot.body, user_id, emotes) {
                let mut m = (**slot).clone();
                m.body = body;
                *slot = Arc::new(m);
                events.push(RoomEvent::Update { message: slot.clone() });
            }
        }
        events
    }

    /// The new status, and a line in chat when the link comes or goes (V101-32).
    pub fn set_status(&mut self, status: ConnStatus, now_ms: i64) -> Vec<RoomEvent> {
        if self.status == status {
            return Vec::new();
        }
        let line = match (self.status, status) {
            (ConnStatus::Reconnecting, ConnStatus::Joined) => Some("Reconnected"),
            (_, ConnStatus::Joined) => Some("Joined chat"),
            (ConnStatus::Joined, ConnStatus::Reconnecting) => Some("Disconnected"),
            _ => None,
        };
        self.status = status;
        let mut events = vec![RoomEvent::Status { status }];
        events.extend(line.map(|text| self.system(NoticeKind::Info, text, now_ms)));
        events
    }

    /// A line of ours, e.g. a send error found before reaching Twitch.
    pub fn system(&mut self, kind: NoticeKind, text: &str, now_ms: i64) -> RoomEvent {
        let m = Arc::new(build::system(kind, text, now_ms));
        self.push(m.clone());
        RoomEvent::Message { message: m }
    }

    /// 1.26 A finished line from outside the room, e.g. a whisper shown inline.
    pub fn local(&mut self, m: Arc<ChatMessage>) -> RoomEvent {
        self.push(m.clone());
        RoomEvent::Message { message: m }
    }

    pub fn sending(&mut self, send: PendingSend) {
        self.expire(send.at_ms);
        self.pending.push_back(send);
    }

    fn expire(&mut self, now_ms: i64) {
        while self.pending.front().is_some_and(|p| now_ms - p.at_ms > PENDING_TTL_MS) {
            self.pending.pop_front();
        }
    }

    pub fn handle_line(&mut self, line: &Line<'_>, now_ms: i64, emotes: &dyn Emotes) -> Vec<RoomEvent> {
        let incoming = {
            let mut ctx = Ctx::new(self.me.as_ref(), &mut self.colors, now_ms);
            ctx.emotes = emotes;
            build::build(line, &mut ctx)
        };
        incoming.map_or_else(Vec::new, |i| self.apply(i, now_ms, emotes))
    }

    pub fn apply(&mut self, incoming: Incoming, now_ms: i64, emotes: &dyn Emotes) -> Vec<RoomEvent> {
        let mut events = Vec::new();
        match incoming {
            Incoming::Message(m) => {
                // V102-21 Our own line from another app carries fresh badges.
                if m.has(Flag::Own)
                    && !m.has(Flag::History)
                    && m.source.is_none()
                    && let Some(s) = &m.sender
                {
                    events.extend(self.own_line(&s.badges));
                }
                events.extend(self.add(m, now_ms, emotes));
            }
            Incoming::Gift { message, gift } => events.extend(self.gift(message, gift, now_ms, emotes)),
            Incoming::Delete { id } => {
                self.mark_deleted(|m| m.id == id);
                events.push(RoomEvent::Delete { id });
            }
            Incoming::ClearUser { user_id, secs, notice } => {
                self.mark_deleted(|m| m.notice.is_none() && m.sender_id() == Some(user_id.as_str()));
                let me = self.me.as_ref().is_some_and(|m| m.user_id == user_id);
                events.push(RoomEvent::ClearUser { user_id });
                events.extend(self.add(notice, now_ms, emotes));
                if me && let Some(secs) = secs {
                    events.extend(self.wait(now_ms + secs as i64 * 1000));
                }
            }
            Incoming::Clear { notice } => {
                events.push(RoomEvent::Cleared);
                events.extend(self.add(notice, now_ms, emotes));
            }
            Incoming::RoomState(update) => {
                update.apply(&mut self.state);
                events.push(RoomEvent::State {
                    state: self.state.clone(),
                });
            }
            Incoming::UserState(us) => {
                if let Some(me) = &self.me {
                    let own = us.to_self(&me.login, self.owner());
                    events.extend(self.set_own(own));
                }
                if let Some(id) = us.sent_id.clone() {
                    self.expire(now_ms);
                    if let Some(p) = self.pending.pop_front() {
                        let m = self.echo(p, &us, id, now_ms, emotes);
                        events.extend(self.add(m, now_ms, emotes));
                        let slow = self.state.slow;
                        let exempt = self.state.own.as_ref().is_some_and(|o| o.fast_lane());
                        if slow > 0 && !exempt {
                            events.extend(self.wait(now_ms + i64::from(slow) * 1000));
                        }
                    }
                }
            }
            Incoming::Notice { msg_id, notice } => {
                if msg_id.starts_with("msg_") {
                    self.expire(now_ms);
                    self.pending.pop_front();
                }
                if msg_id == "msg_channel_suspended" {
                    events.extend(self.set_status(ConnStatus::Unavailable, now_ms));
                }
                if matches!(msg_id.as_str(), "msg_slowmode" | "msg_timedout")
                    && let Some(secs) = seconds_in(notice.notice.as_ref().map_or("", |n| n.text.as_str()))
                {
                    events.extend(self.wait(now_ms + secs * 1000));
                }
                events.extend(self.add(notice, now_ms, emotes));
            }
            Incoming::Presence { login, joined } => events.extend(self.presence(login, joined, now_ms)),
        }
        events
    }

    /// C1 Lines from the history service, added before everything live.
    pub fn history(&mut self, lines: &[String], now_ms: i64, emotes: &dyn Emotes) -> Vec<Arc<ChatMessage>> {
        let known: HashSet<String> = self.buffer.iter().map(|m| m.id.clone()).collect();
        let mut old: Vec<ChatMessage> = Vec::new();
        for raw in lines {
            let Some(line) = Line::parse(raw) else { continue };
            let incoming = {
                let mut ctx = Ctx::new(self.me.as_ref(), &mut self.colors, now_ms);
                ctx.emotes = emotes;
                build::build(&line, &mut ctx)
            };
            match incoming {
                Some(
                    Incoming::Message(mut m)
                    | Incoming::Clear { notice: mut m }
                    | Incoming::Gift { message: mut m, .. },
                ) => {
                    m.set(Flag::History);
                    if !known.contains(&m.id) && self.hooked(&mut m, emotes) && self.screen_old(&mut m) {
                        old.push(m);
                    }
                }
                Some(Incoming::ClearUser { user_id, notice: mut n, .. }) => {
                    for m in &mut old {
                        if m.notice.is_none() && m.sender_id() == Some(user_id.as_str()) {
                            m.set(Flag::Deleted);
                        }
                    }
                    n.set(Flag::History);
                    old.push(n);
                }
                Some(Incoming::Delete { id }) => {
                    if let Some(m) = old.iter_mut().find(|m| m.id == id) {
                        m.set(Flag::Deleted);
                    }
                }
                _ => {}
            }
        }
        let room = self.keep.saturating_sub(self.buffer.len());
        let skip = old.len().saturating_sub(room);
        let old: Vec<Arc<ChatMessage>> = old.into_iter().skip(skip).map(Arc::new).collect();
        for m in old.iter().rev() {
            self.buffer.push_front(m.clone());
        }
        old
    }

    /// C16 Adds a single gift to its mass gift's line, or the notice as its own line.
    fn gift(&mut self, message: ChatMessage, gift: GiftLink, now_ms: i64, emotes: &dyn Emotes) -> Option<RoomEvent> {
        self.gifts.retain(|g| g.expires_ms > now_ms);
        match gift {
            GiftLink::Mass { origin, count } => {
                self.gifts.push(GiftGroup {
                    origin,
                    message_id: message.id.clone(),
                    size: count,
                    expires_ms: now_ms + GIFT_MERGE_MS,
                });
                self.add(message, now_ms, emotes)
            }
            GiftLink::Single {
                community: Some(origin),
                recipient,
            } => match self.merge_gift(&origin, recipient) {
                Some(ev) => Some(ev),
                None => self.add(message, now_ms, emotes),
            },
            GiftLink::Single { community: None, .. } => self.add(message, now_ms, emotes),
        }
    }

    fn merge_gift(&mut self, origin: &str, recipient: String) -> Option<RoomEvent> {
        let g = self.gifts.iter().position(|g| g.origin == origin)?;
        let id = self.gifts[g].message_id.clone();
        let slot = self.buffer.iter_mut().find(|m| m.id == id)?;
        let mut m = (**slot).clone();
        let notice = m.notice.as_mut()?;
        notice.recipients.push(recipient);
        let full = notice.recipients.len() >= self.gifts[g].size;
        let m = Arc::new(m);
        *slot = m.clone();
        if full {
            self.gifts.remove(g);
        }
        Some(RoomEvent::Update { message: m })
    }

    /// C6 We can't talk until `until_ms`. Only moves the time forward.
    fn wait(&mut self, until_ms: i64) -> Option<RoomEvent> {
        self.state.wait_until.is_none_or(|w| w < until_ms).then(|| {
            self.state.wait_until = Some(until_ms);
            RoomEvent::State {
                state: self.state.clone(),
            }
        })
    }

    /// A live message, unless the user's rules hide it.
    fn add(&mut self, mut m: ChatMessage, now_ms: i64, emotes: &dyn Emotes) -> Option<RoomEvent> {
        if let Some(r) = m.reward.as_mut().filter(|r| r.title.is_none())
            && let Some((title, cost)) = self.rewards.get(&r.id)
        {
            r.title = Some(title.clone());
            r.cost = Some(*cost);
        }
        if !self.hooked(&mut m, emotes) {
            return None;
        }
        let (shown, outcome) = self.screen(&mut m, now_ms);
        let m = Arc::new(m);
        if outcome.sound || outcome.flash || !outcome.routes.is_empty() {
            self.outcomes.push((m.clone(), outcome));
        }
        if !shown {
            return None;
        }
        if let Some(ev) = self.collapse(&m) {
            return Some(ev);
        }
        self.push(m.clone());
        Some(RoomEvent::Message { message: m })
    }

    fn push(&mut self, m: Arc<ChatMessage>) {
        self.buffer.push_back(m);
        self.trim();
    }

    fn mark_deleted(&mut self, hit: impl Fn(&ChatMessage) -> bool) {
        for slot in self.buffer.iter_mut() {
            if hit(slot) && !slot.has(Flag::Deleted) {
                let mut m = (**slot).clone();
                m.set(Flag::Deleted);
                *slot = Arc::new(m);
            }
        }
    }

    /// Twitch doesn't echo our messages; USERSTATE with an id confirms one. Built from
    /// what we sent plus our badges and color in this room.
    fn echo(&mut self, p: PendingSend, us: &UserState, id: String, now_ms: i64, emotes: &dyn Emotes) -> ChatMessage {
        let tokens = tokenize(
            &Input {
                text: &p.text,
                sender: self.me.as_ref().map_or("", |m| m.user_id.as_str()),
                emotes: "",
                gifs: "",
                bits: 0,
            },
            &[],
            emotes,
        );
        let sender = self.me.as_ref().map(|me| Sender {
            id: me.user_id.clone(),
            login: me.login.clone(),
            display: us.display.clone().unwrap_or_else(|| me.login.clone()),
            color: self.colors.get(
                us.color
                    .as_deref()
                    .unwrap_or_else(|| build::sender::default_color(&me.user_id, &me.login)),
            ),
            badges: us.badges.clone(),
        });
        let mut flags = vec![Flag::Own];
        if p.action {
            flags.push(Flag::Action);
        }
        ChatMessage {
            id,
            ts: now_ms,
            sender,
            body: tokens.body,
            notice: None,
            reply: p.reply,
            reward: None,
            source: None,
            flags,
            meta: Meta::default(),
        }
    }
}

/// "…talk again in 10 seconds." or "…for 600 more seconds." → the number.
fn seconds_in(text: &str) -> Option<i64> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let at = words.iter().position(|w| w.starts_with("second"))?;
    words[at.saturating_sub(2)..at].iter().rev().find_map(|w| w.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::plain;
    use crate::tokenize::NoEmotes;

    fn room() -> Room {
        Room::new("Chan", Some(Me::new("42", "me", "Me")))
    }

    fn feed(r: &mut Room, raw: &str) -> Vec<RoomEvent> {
        r.handle_line(&Line::parse(raw).unwrap(), 1_000, &NoEmotes)
    }

    fn msg(id: &str, user: &str) -> String {
        format!("@id={id};user-id={user};tmi-sent-ts=5 :u{user}!u@u PRIVMSG #chan :hello {id}")
    }

    #[test]
    fn buffer_is_capped() {
        let mut r = room();
        for i in 0..BUFFER + 10 {
            feed(&mut r, &msg(&i.to_string(), "7"));
        }
        let s = r.snapshot();
        assert_eq!(s.messages.len(), BUFFER);
        assert_eq!(s.messages[0].id, "10");
    }

    #[test]
    fn scrollback_raises_the_cap_but_never_below_the_floor() {
        let mut r = room();
        r.set_keep(1_000);
        for i in 0..1_005 {
            feed(&mut r, &msg(&i.to_string(), "7"));
        }
        assert_eq!(r.snapshot().messages.len(), 1_000);
        r.set_keep(100);
        assert_eq!(r.keep(), BUFFER);
        let s = r.snapshot();
        assert_eq!(s.messages.len(), BUFFER);
        assert_eq!(s.messages[0].id, "505");
    }

    #[test]
    fn link_changes_leave_a_line() {
        let mut r = room();
        let text = |evs: &[RoomEvent]| -> Vec<String> {
            evs.iter()
                .filter_map(|e| match e {
                    RoomEvent::Message { message } => message.notice.as_ref().map(|n| n.text.clone()),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(text(&r.set_status(ConnStatus::Joined, 1)), ["Joined chat"]);
        assert!(r.set_status(ConnStatus::Joined, 2).is_empty());
        assert_eq!(text(&r.set_status(ConnStatus::Reconnecting, 3)), ["Disconnected"]);
        assert_eq!(text(&r.set_status(ConnStatus::Joined, 4)), ["Reconnected"]);
        let auth = r.set_status(ConnStatus::AuthFailed, 5);
        assert!(matches!(auth.as_slice(), [RoomEvent::Status { status: ConnStatus::AuthFailed }]));
        assert_eq!(r.snapshot().messages.len(), 3);
    }

    #[test]
    fn deletions_and_timeouts_mark_messages() {
        let mut r = room();
        feed(&mut r, &msg("a", "7"));
        feed(&mut r, &msg("b", "7"));
        feed(&mut r, &msg("c", "8"));
        feed(&mut r, "@target-msg-id=c :tmi.twitch.tv CLEARMSG #chan :hello c");
        let ev = feed(&mut r, "@ban-duration=60;target-user-id=7 :tmi.twitch.tv CLEARCHAT #chan :u7");
        assert!(matches!(&ev[0], RoomEvent::ClearUser { user_id } if user_id == "7"));
        let s = r.snapshot();
        assert!(s.messages[..3].iter().all(|m| m.has(Flag::Deleted)));
        assert_eq!(s.messages[3].notice.as_ref().unwrap().kind, NoticeKind::Moderation);
    }

    #[test]
    fn own_messages_are_echoed_from_userstate() {
        let mut r = room();
        feed(&mut r, "@badges=moderator/1;mod=1 :tmi.twitch.tv USERSTATE #chan");
        assert!(r.privileged());
        r.sending(PendingSend {
            text: "hi there".into(),
            action: false,
            reply: None,
            at_ms: 900,
        });
        let ev = feed(&mut r, "@badges=moderator/1;color=#FF0000;display-name=Me;id=sent1;mod=1 :tmi.twitch.tv USERSTATE #chan");
        let RoomEvent::Message { message } = &ev[0] else { panic!("{ev:?}") };
        assert_eq!((message.id.as_str(), plain(&message.body).as_str()), ("sent1", "hi there"));
        assert!(message.has(Flag::Own));
        assert_eq!(message.sender.as_ref().unwrap().badges[0].set, "moderator");
    }

    #[test]
    fn a_refused_send_is_dropped_and_explained() {
        let mut r = room();
        r.sending(PendingSend {
            text: "same".into(),
            action: false,
            reply: None,
            at_ms: 900,
        });
        let ev = feed(&mut r, "@msg-id=msg_duplicate :tmi.twitch.tv NOTICE #chan :Your message was not sent");
        assert_eq!(ev.len(), 1);
        // A later USERSTATE with an id has nothing to echo.
        let ev = feed(&mut r, "@id=x :tmi.twitch.tv USERSTATE #chan");
        assert!(ev.iter().all(|e| !matches!(e, RoomEvent::Message { .. })));
    }

    #[test]
    fn history_goes_first_without_duplicates() {
        let mut r = room();
        feed(&mut r, &msg("live", "7"));
        let lines = [
            msg("h1", "7"),
            msg("h2", "8"),
            msg("live", "7"),
            "@ban-duration=5;target-user-id=8 :tmi.twitch.tv CLEARCHAT #chan :u8".to_owned(),
        ];
        let added = r.history(&lines, 1_000, &NoEmotes);
        assert_eq!(added.len(), 3);
        let ids: Vec<_> = r.snapshot().messages.iter().map(|m| m.id.clone()).collect();
        assert_eq!(ids[..2], ["h1", "h2"]);
        assert_eq!(ids.last().unwrap(), "live");
        let s = r.snapshot();
        assert!(s.messages[1].has(Flag::Deleted) && s.messages[1].has(Flag::History));
        assert!(!s.messages[0].has(Flag::Deleted));
    }

    #[test]
    fn gift_subs_merge_into_their_mass_gift() {
        let mut r = room();
        let mass = r"@id=g0;login=a;msg-id=submysterygift;msg-param-mass-gift-count=2;msg-param-origin-id=77;system-msg=A\sis\sgifting\s2;user-id=9 :tmi.twitch.tv USERNOTICE #chan";
        let one = |n: &str| {
            format!(
                r"@id=g{n};login=a;msg-id=subgift;msg-param-community-gift-id=77;msg-param-origin-id=77;msg-param-recipient-display-name=R{n};system-msg=A\sgifted;user-id=9 :tmi.twitch.tv USERNOTICE #chan"
            )
        };
        feed(&mut r, mass);
        let ev = feed(&mut r, &one("1"));
        assert!(
            matches!(&ev[0], RoomEvent::Update { message } if message.notice.as_ref().unwrap().recipients == ["R1"])
        );
        feed(&mut r, &one("2"));
        // The group is full: a third one stands alone.
        let ev = feed(&mut r, &one("3"));
        assert!(matches!(&ev[0], RoomEvent::Message { .. }));
        let s = r.snapshot();
        assert_eq!(s.messages.len(), 2);
        assert_eq!(s.messages[0].notice.as_ref().unwrap().recipients, ["R1", "R2"]);
    }

    #[test]
    fn cooldowns_from_slow_mode_and_timeouts() {
        let mut r = room();
        feed(&mut r, "@slow=10;room-id=1 :tmi.twitch.tv ROOMSTATE #chan");
        r.sending(PendingSend {
            text: "x".into(),
            action: false,
            reply: None,
            at_ms: 1_000,
        });
        feed(&mut r, "@id=s1 :tmi.twitch.tv USERSTATE #chan");
        assert_eq!(r.state.wait_until, Some(11_000));
        feed(
            &mut r,
            "@msg-id=msg_timedout :tmi.twitch.tv NOTICE #chan :You are timed out for 600 more seconds.",
        );
        assert_eq!(r.state.wait_until, Some(601_000));
        feed(&mut r, "@ban-duration=60;target-user-id=42 :tmi.twitch.tv CLEARCHAT #chan :me");
        assert_eq!(r.state.wait_until, Some(601_000), "never moves back");
        assert_eq!(seconds_in("You will be able to talk again in 3 seconds."), Some(3));
    }

    #[test]
    fn redemptions_get_their_title_and_replies_their_header() {
        let mut r = room();
        r.set_rewards(HashMap::from([("r1".to_owned(), ("Hydrate".to_owned(), 500))]));
        feed(&mut r, "@id=m1;user-id=7;custom-reward-id=r1 :u!u@u PRIVMSG #chan :drink");
        let s = r.snapshot();
        let reward = s.messages[0].reward.as_ref().unwrap();
        assert_eq!((reward.title.as_deref(), reward.cost), (Some("Hydrate"), Some(500)));
        let reply = r.reply_to("m1").unwrap();
        assert_eq!((reply.parent_body.as_str(), reply.thread_id.as_str()), ("drink", "m1"));
        assert!(r.reply_to("nope").is_none());
    }

    #[test]
    fn recent_messages_of_a_user() {
        let mut r = room();
        for i in 0..15 {
            feed(&mut r, &msg(&format!("m{i}"), if i % 2 == 0 { "7" } else { "8" }));
        }
        let last = r.recent_from("7", 3);
        assert_eq!(last.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["m10", "m12", "m14"]);
    }
}
