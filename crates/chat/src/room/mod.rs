//! One joined channel (1.8): message buffer, room state, our sends and their echoes,
//! deletions, history (C1), the user's rules (`checks`). Pure state: the app feeds it
//! lines and streams the events.

mod checks;
mod gap;
mod hook;
mod keep;
mod presence;
mod role;
mod stack;

pub use hook::{LineChange, LineHook};
pub use keep::{BACKGROUND, BACKGROUND_HISTORY};

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
    /// V103-04 The pin was read since the room opened, so `pinned: None` means there is
    /// none rather than not known yet.
    pub pin_known: bool,
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
    /// V107-M1 No view shows the room: it keeps only `keep::BACKGROUND`.
    background: bool,
    /// V107-M1 When the first live line came, so refilled lines from then on aren't dimmed.
    live_from: Option<i64>,
    pending: VecDeque<PendingSend>,
    gifts: Vec<GiftGroup>,
    /// 1.9 Reward titles and costs by id, to name redemptions.
    rewards: HashMap<String, (String, u32)>,
    pinned: Option<Pinned>,
    pin_known: bool,
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
    /// V107-M7 Timeouts in a row, one line per user.
    timeouts: stack::Stacks,
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
            background: false,
            live_from: None,
            pending: VecDeque::new(),
            gifts: Vec::new(),
            rewards: HashMap::new(),
            pinned: None,
            pin_known: false,
            colors: Colors::shared(),
            rules: Arc::default(),
            hook: None,
            repeats: Repeats::default(),
            outcomes: Vec::new(),
            lost_at: None,
            presence: presence::Batches::default(),
            timeouts: stack::Stacks::default(),
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
        self.snapshot_tail(usize::MAX)
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
        let over = self.buffer.len().saturating_sub(self.limit());
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
            pin_known: self.pin_known,
        }
    }

    pub fn colors(&mut self) -> &mut Colors {
        &mut self.colors
    }

    /// 1.9 Names future redemptions from the channel's reward list.
    pub fn set_rewards(&mut self, rewards: HashMap<String, (String, u32)>) {
        self.rewards = rewards;
    }

    /// Returns an event if the pin changed, and on the first read: "nothing pinned" is news
    /// then too (V103-04).
    pub fn set_pinned(&mut self, pin: Option<Pinned>) -> Option<RoomEvent> {
        let first = !std::mem::replace(&mut self.pin_known, true);
        (first || self.pinned != pin).then(|| {
            self.pinned = pin.clone();
            RoomEvent::Pinned { pin: pin.map(Box::new) }
        })
    }

    /// V107-P4 Message `id`, from the buffer or the pin.
    fn shown(&self, id: &str) -> Option<&ChatMessage> {
        let pinned = self.pinned.as_ref().map(|p| &p.message).filter(|m| m.id == id);
        self.buffer.iter().rev().find(|m| m.id == id).map(|m| &**m).or(pinned)
    }

    /// V107-P4 Who sent message `id` (their display name), from the buffer or the pin.
    pub fn sender_name(&self, id: &str) -> Option<String> {
        self.shown(id).and_then(|m| m.sender.as_ref().map(|s| s.display.clone()))
    }

    /// V107-P7 Message `id`'s plain text (emotes as their names), for the delete notice.
    pub fn message_text(&self, id: &str) -> Option<String> {
        self.shown(id).map(|m| plain(&m.body))
    }

    /// V107 Whether message `id` is the one pinned here.
    pub fn pins(&self, id: &str) -> bool {
        self.pinned.as_ref().is_some_and(|p| p.message.id == id)
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

    /// V106-08 The other channels of a shared chat: `source-room-id`s of lines since `since_ms`.
    pub fn shared_rooms(&self, since_ms: i64) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for m in self.buffer.iter().rev().take_while(|m| m.ts >= since_ms) {
            if let Some(s) = &m.source
                && !out.contains(&s.room_id)
            {
                out.push(s.room_id.clone());
            }
        }
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

    /// V105-05 Our JOIN went unanswered: the chat counts as refused, unless it got in or
    /// went away for another reason meanwhile.
    pub fn join_unanswered(&mut self, now_ms: i64) -> Vec<RoomEvent> {
        if self.waiting() {
            self.set_status(ConnStatus::Refused, now_ms)
        } else {
            Vec::new()
        }
    }

    /// V105-05 Our JOIN is out and Twitch hasn't confirmed it. A suspended, refused or
    /// rejected chat isn't waiting anymore.
    fn waiting(&self) -> bool {
        matches!(self.status, ConnStatus::Connecting | ConnStatus::Reconnecting)
    }

    /// A line of ours, e.g. a send error found before reaching Twitch.
    pub fn system(&mut self, kind: NoticeKind, text: &str, now_ms: i64) -> RoomEvent {
        let m = Arc::new(build::system(kind, text, now_ms));
        self.push(m.clone());
        RoomEvent::Message { message: m }
    }

    /// V107-M7 A mod-action feed's timeout line for `user_id`, stacked with the ones
    /// before it (`stack`).
    pub fn timeout_line(&mut self, user_id: &str, text: &str, now_ms: i64) -> RoomEvent {
        let mut m = build::system(NoticeKind::Moderation, text, now_ms);
        if let Some(ev) = self.stacked(user_id, stack::Source::Feed, &mut m) {
            return ev;
        }
        let m = Arc::new(m);
        self.push(m.clone());
        RoomEvent::Message { message: m }
    }

    /// V107-M7 Puts timeout line `m` into its stack: the update of the stack's line, or
    /// `None` to add `m` (its text counted) as a new line.
    fn stacked(&mut self, user_id: &str, src: stack::Source, m: &mut ChatMessage) -> Option<RoomEvent> {
        let text = m.notice.as_ref()?.text.clone();
        let mut hit = self.timeouts.hit(user_id, src, &text, m.ts, &m.id);
        if hit.update {
            if let Some(slot) = self.buffer.iter_mut().rev().find(|b| b.id == hit.id) {
                let mut u = (**slot).clone();
                if let Some(n) = u.notice.as_mut() {
                    n.text = hit.text;
                }
                *slot = Arc::new(u);
                return Some(RoomEvent::Update { message: slot.clone() });
            }
            // The stack's line is gone: this one starts a new stack.
            self.timeouts.forget(user_id);
            hit = self.timeouts.hit(user_id, src, &text, m.ts, &m.id);
        }
        if let Some(n) = m.notice.as_mut() {
            n.text = hit.text;
        }
        None
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
                // V105-06 Our line as Twitch relayed it replaces the echo built from USERSTATE.
                let echoed = (m.has(Flag::Own) && !m.has(Flag::History))
                    .then(|| self.buffer.iter().position(|b| b.id == m.id))
                    .flatten();
                match echoed {
                    Some(i) => events.extend(self.upgrade(i, m)),
                    None => events.extend(self.add(m, now_ms, emotes)),
                }
            }
            Incoming::Gift { message, gift } => events.extend(self.gift(message, gift, now_ms, emotes)),
            Incoming::Delete { id } => {
                self.mark_deleted(|m| m.id == id);
                events.push(RoomEvent::Delete { id });
            }
            Incoming::ClearUser { user_id, secs, notice } => {
                self.mark_deleted(|m| m.notice.is_none() && m.sender_id() == Some(user_id.as_str()));
                let me = self.me.as_ref().is_some_and(|m| m.user_id == user_id);
                // V107-M7 Timeouts in a row share one line; bans stand alone.
                let mut notice = notice;
                let update = secs.and_then(|_| self.stacked(&user_id, stack::Source::Irc, &mut notice));
                events.push(RoomEvent::ClearUser { user_id });
                match update {
                    Some(update) => events.push(update),
                    None => events.extend(self.add(notice, now_ms, emotes)),
                }
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
                        // V105-06 The reader may have brought our line already.
                        if !self.buffer.iter().any(|b| b.id == id) {
                            let m = self.echo(p, &us, id, now_ms, emotes);
                            events.extend(self.add(m, now_ms, emotes));
                        }
                        let slow = self.state.slow;
                        let exempt = self.state.own.as_ref().is_some_and(|o| o.fast_lane());
                        if slow > 0 && !exempt {
                            events.extend(self.wait(now_ms + i64::from(slow) * 1000));
                        }
                    }
                }
            }
            Incoming::Notice { msg_id, notice } => {
                // A send's answer (V105-06 the writer's NOTICE for a pending line).
                let answered = msg_id.starts_with("msg_") && {
                    self.expire(now_ms);
                    self.pending.pop_front().is_some()
                };
                if msg_id == "msg_channel_suspended" {
                    events.extend(self.set_status(ConnStatus::Unavailable, now_ms));
                } else if !answered && self.me.is_some() && self.waiting() {
                    // V105-05 Twitch spoke instead of letting us in (Chatterino shows such notices
                    // where they're aimed; here the chat also counts as not joined).
                    events.extend(self.set_status(ConnStatus::Refused, now_ms));
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
        self.older(lines, None, now_ms, emotes)
    }

    /// History lines put in above the buffer. `before`: only lines older than this (V107-M1).
    fn older(&mut self, lines: &[String], before: Option<i64>, now_ms: i64, emotes: &dyn Emotes) -> Vec<Arc<ChatMessage>> {
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
                    if before.is_some_and(|b| m.ts >= b) {
                        continue;
                    }
                    self.mark_old(&mut m, before.is_some());
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
                    if before.is_none_or(|b| n.ts < b) {
                        self.mark_old(&mut n, before.is_some());
                        old.push(n);
                    }
                }
                Some(Incoming::Delete { id }) => {
                    if let Some(m) = old.iter_mut().find(|m| m.id == id) {
                        m.set(Flag::Deleted);
                    }
                }
                _ => {}
            }
        }
        let room = self.limit().saturating_sub(self.buffer.len());
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
        // V106-33 A pop-up alone counts too.
        if outcome.pings() || !outcome.routes.is_empty() {
            self.outcomes.push((m.clone(), outcome));
        }
        if !shown {
            return None;
        }
        if let Some(ev) = self.collapse(&m) {
            return Some(ev);
        }
        if m.sender.is_some() {
            self.live_from.get_or_insert(m.ts);
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

    /// V105-06 Our line as Twitch relayed it, in place of the echo built from USERSTATE: same
    /// spot and time, now with Twitch's tags (first message, reply, emote positions). Like
    /// history, it's highlighted without sounds, and plugins don't see it twice.
    fn upgrade(&mut self, i: usize, mut m: ChatMessage) -> Option<RoomEvent> {
        if !self.screen_old(&mut m) {
            return None;
        }
        let old = &self.buffer[i];
        m.ts = old.ts;
        for f in [Flag::Similar, Flag::Deleted] {
            if old.has(f) {
                m.set(f);
            }
        }
        m.meta.repeat = old.meta.repeat;
        m.meta.translation = old.meta.translation.clone();
        m.meta.plugin = old.meta.plugin.clone();
        let m = Arc::new(m);
        self.buffer[i] = m.clone();
        Some(RoomEvent::Update { message: m })
    }

    /// Twitch doesn't echo our messages to the connection that sent them; USERSTATE with an
    /// id confirms one. Shown at once, built from what we sent plus our badges and color in
    /// this room; the reader's copy replaces it (V105-06).
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
mod tests;

#[cfg(test)]
mod popup_tests;
