//! The user's rules on arriving messages (2.6, F2, F6, C2), see `crate::rules`.

use std::sync::Arc;

use super::Room;
use crate::model::{ChatMessage, Flag, RoomEvent, Translation, plain};
use crate::rules::when::Ctx;
use crate::rules::{Outcome, Rules};
use crate::similar::{SimilarCheck, dice, lcs_ratio, normalize};

impl Room {
    /// New rules apply to messages from now on.
    pub fn set_rules(&mut self, rules: Arc<Rules>) {
        self.rules = rules;
    }

    /// V109-C01 Follows or unfollows the thread that starts at message `thread`.
    pub fn follow_thread(&mut self, thread: &str, on: bool) {
        if on {
            self.followed.insert(thread.to_owned());
        } else {
            self.followed.remove(thread);
        }
    }

    pub fn follows_thread(&self, thread: &str) -> bool {
        self.followed.contains(thread)
    }

    /// V109-C01 Whether `m` is a reply inside a followed thread.
    fn in_followed_thread(&self, m: &ChatMessage) -> bool {
        !self.followed.is_empty() && m.reply.as_ref().is_some_and(|r| self.followed.contains(&r.thread_id))
    }

    /// What arrived messages want from the app: sounds, the taskbar, feed tabs.
    pub fn take_outcomes(&mut self) -> Vec<(Arc<ChatMessage>, Outcome)> {
        std::mem::take(&mut self.outcomes)
    }

    /// C17 Puts a translation on a message that's still here.
    pub fn set_translation(&mut self, id: &str, t: Translation) -> Option<RoomEvent> {
        let slot = self.buffer.iter_mut().rev().find(|m| m.id == id)?;
        let mut m = (**slot).clone();
        m.meta.translation = Some(t);
        *slot = Arc::new(m);
        Some(RoomEvent::Update {
            message: slot.clone(),
        })
    }

    /// The text of a message, for translating. `None` if it's gone or has none.
    pub fn text_of(&self, id: &str) -> Option<String> {
        let m = self.buffer.iter().rev().find(|m| m.id == id)?;
        let text: Vec<&str> = m
            .body
            .iter()
            .filter_map(|f| match f {
                crate::model::Fragment::Text { text } => Some(text.trim()),
                _ => None,
            })
            .filter(|t| !t.is_empty())
            .collect();
        (!text.is_empty()).then(|| text.join(" "))
    }

    /// V110-C59 What the app's live check last said: live or not, and the category.
    pub fn set_stream(&mut self, live: bool, category: Option<&str>) {
        self.stream = Some((live, category.filter(|_| live).map(str::to_lowercase)));
    }

    /// F2, V110-C59 What rules see of this room for `m`: our role (V102-21's one check),
    /// live state, category. `recent`: the sender's message times, live lines only.
    fn ctx<'a>(&'a self, m: &ChatMessage, recent: Option<&'a std::collections::VecDeque<i64>>) -> Ctx<'a> {
        let own = self.state.own.as_ref();
        Ctx {
            channel: &self.login,
            live: self.stream.as_ref().map(|s| s.0),
            category: self.stream.as_ref().and_then(|s| s.1.as_deref()),
            role: own.and_then(|o| o.role),
            vip: own.is_some_and(|o| o.vip),
            first: m.has(Flag::FirstMessage),
            ts: m.ts,
            recent,
        }
    }

    /// V110-C59 Counts the sender's line when some rule asks for bursts.
    fn note_burst(&mut self, m: &ChatMessage) {
        if let (Some(keep), Some(s)) = (self.rules.burst(), m.sender.as_ref())
            && m.notice.is_none()
        {
            self.bursts.note(&s.id, m.ts, keep);
        }
    }

    /// A live message: whether it shows, and what it asks for.
    pub(super) fn screen(&mut self, m: &mut ChatMessage, now_ms: i64) -> (bool, Outcome) {
        let rules = self.rules.clone();
        let mut routes = Vec::new();
        self.note_burst(m);
        let recent = rules.burst().and_then(|_| self.bursts.get(m.sender_id()?));
        let ctx = self.ctx(m, recent);
        if m.sender.is_some() && rules.hidden(m, &ctx, &mut routes) {
            return (
                false,
                Outcome {
                    routes,
                    ..Outcome::default()
                },
            );
        }
        rules.replace(m, &ctx);
        if m.notice.is_none()
            && let Some(sender) = m.sender.as_ref().map(|s| s.id.clone())
        {
            let own = m.has(Flag::Own);
            if let Some(check) = &rules.similar
                && (!own || rules.similar_own)
                && self.similar(m, check)
            {
                m.set(Flag::Similar);
            }
            if let Some(check) = &rules.repeat
                && !own
            {
                m.meta.repeat = self.repeats.count(check, &sender, &plain(&m.body), now_ms);
            }
        }
        // V109-C01 A new line in a followed thread counts as a reply to you (the Replies row).
        if m.notice.is_none() && !m.has(Flag::Own) && self.in_followed_thread(m) {
            m.set(Flag::RepliesToMe);
        }
        let recent = rules.burst().and_then(|_| self.bursts.get(m.sender_id()?));
        let ctx = self.ctx(m, recent);
        let mut outcome = rules.highlight(m, &ctx);
        // Near-duplicates don't ping (Chatterino).
        if m.has(Flag::Similar) {
            outcome.sound = false;
            outcome.flash = false;
            outcome.notify = false;
        }
        (true, outcome)
    }

    /// History: hidden ones are left out, the rest highlighted, without sounds.
    pub(super) fn screen_old(&mut self, m: &mut ChatMessage) -> bool {
        let rules = self.rules.clone();
        let ctx = self.ctx(m, None);
        if m.sender.is_some() && rules.hidden(m, &ctx, &mut Vec::new()) {
            return false;
        }
        rules.replace(m, &ctx);
        rules.highlight(m, &ctx);
        true
    }

    /// C2 A repeat straight after the sender's own line counts up on that line instead
    /// of adding another.
    pub(super) fn collapse(&mut self, m: &ChatMessage) -> Option<RoomEvent> {
        let n = m.meta.repeat?;
        let check = self.rules.repeat.clone()?;
        let last = self.buffer.back_mut()?;
        if last.notice.is_some() || last.has(Flag::Deleted) || last.sender_id() != m.sender_id() {
            return None;
        }
        let (a, b) = (normalize(&plain(&last.body)), normalize(&plain(&m.body)));
        if a != b && dice(&a, &b) < check.threshold {
            return None;
        }
        let mut merged = (**last).clone();
        merged.meta.repeat = Some(n);
        *last = Arc::new(merged);
        Some(RoomEvent::Update {
            message: last.clone(),
        })
    }

    /// F6 Close to one of the sender's last few messages (or anyone's).
    fn similar(&self, m: &ChatMessage, check: &SimilarCheck) -> bool {
        let text = plain(&m.body);
        let sender = m.sender_id();
        let mut compared = 0;
        for old in self.buffer.iter().rev() {
            if m.ts - old.ts > check.max_age_ms {
                break;
            }
            if old.notice.is_some() || old.sender.is_none() || (check.same_user && old.sender_id() != sender)
            {
                continue;
            }
            if lcs_ratio(&text, &plain(&old.body)) > check.threshold {
                return true;
            }
            compared += 1;
            if compared >= check.depth {
                break;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod when_tests;
