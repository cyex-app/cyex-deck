//! The user's rules on arriving messages (2.6, F2, F6, C2), see `crate::rules`.

use std::sync::Arc;

use super::Room;
use crate::model::{ChatMessage, Flag, ModRole, RoomEvent, Translation, plain};
use crate::rules::{Outcome, Role, Rules};
use crate::similar::{SimilarCheck, dice, lcs_ratio, normalize};

impl Room {
    /// New rules apply to messages from now on.
    pub fn set_rules(&mut self, rules: Arc<Rules>) {
        self.rules = rules;
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

    /// F2 Rules for blocked users depend on our role (V102-21's one check).
    fn rules_role(&self) -> Role {
        match self.role() {
            Some(ModRole::Broadcaster) => Role::Broadcaster,
            Some(_) => Role::Moderator,
            None => Role::Viewer,
        }
    }

    /// A live message: whether it shows, and what it asks for.
    pub(super) fn screen(&mut self, m: &mut ChatMessage, now_ms: i64) -> (bool, Outcome) {
        let rules = self.rules.clone();
        let mut routes = Vec::new();
        if m.sender.is_some() && rules.hidden(m, self.rules_role(), &mut routes) {
            return (
                false,
                Outcome {
                    routes,
                    ..Outcome::default()
                },
            );
        }
        rules.replace(m);
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
        let mut outcome = rules.highlight(m);
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
        if m.sender.is_some() && rules.hidden(m, self.rules_role(), &mut Vec::new()) {
            return false;
        }
        rules.replace(m);
        rules.highlight(m);
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
mod tests {
    use super::*;
    use crate::build::Me;
    use crate::irc::Line;
    use crate::model::RoomEvent;
    use crate::similar::RepeatCheck;
    use crate::tokenize::NoEmotes;
    use cyex_settings::schema::{Highlights, IgnoreRule, MentionAlert, MentionRule, Mentions, PhraseRule, Similarity};

    fn feed(r: &mut Room, id: &str, user: &str, text: &str) -> Vec<RoomEvent> {
        let raw = format!("@id={id};user-id={user};tmi-sent-ts={id}000 :u{user}!u@u PRIVMSG #chan :{text}");
        r.handle_line(&Line::parse(&raw).unwrap(), 1_000, &NoEmotes)
    }

    #[test]
    fn rules_hide_replace_highlight_and_flag() {
        let h = Highlights {
            phrases: vec![PhraseRule {
                pattern: "pizza".into(),
                alert: cyex_settings::schema::Alert {
                    sound: true,
                    route: "Food".into(),
                    ..Default::default()
                },
                ..PhraseRule::default()
            }],
            ignores: vec![
                IgnoreRule {
                    pattern: "spam".into(),
                    block: true,
                    route: "Spam".into(),
                    ..IgnoreRule::default()
                },
                IgnoreRule {
                    pattern: "darn".into(),
                    ..IgnoreRule::default()
                },
            ],
            similar: Similarity {
                enabled: true,
                ..Similarity::default()
            },
            ..Highlights::default()
        };
        let mentions = Mentions {
            list: vec![MentionRule {
                pattern: "pasta".into(),
                alert: MentionAlert {
                    notify: true,
                    ..MentionAlert::default()
                },
                ..MentionRule::default()
            }],
            ..Mentions::default()
        };
        let mut rules = Rules::new(&h, &mentions);
        rules.repeat = Some(RepeatCheck {
            threshold: 0.8,
            min: 2,
        });
        let mut r = Room::new("chan", Some(Me::new("42", "me", "Me")));
        r.set_rules(Arc::new(rules));

        assert!(feed(&mut r, "1", "7", "buy spam now").is_empty());
        let out = r.take_outcomes();
        assert_eq!(out[0].1.routes, ["Spam"]);

        let ev = feed(&mut r, "2", "7", "darn I want pizza");
        let RoomEvent::Message { message } = &ev[0] else {
            panic!()
        };
        assert_eq!(plain(&message.body), "*** I want pizza");
        assert!(message.meta.highlight.is_some());
        let out = r.take_outcomes();
        assert!(out[0].1.sound && out[0].1.routes == ["Food"]);

        let ev = feed(&mut r, "3", "7", "darn I want pizza!");
        let RoomEvent::Update { message } = &ev[0] else {
            panic!("{ev:?}")
        };
        assert_eq!(
            (message.id.as_str(), message.meta.repeat),
            ("2", Some(2)),
            "counted on the line before"
        );
        assert!(!r.take_outcomes()[0].1.sound, "near-duplicates don't ping");
        feed(&mut r, "4", "8", "something");
        let ev = feed(&mut r, "5", "7", "darn I want pizza");
        let RoomEvent::Message { message } = &ev[0] else {
            panic!()
        };
        assert_eq!(message.meta.repeat, Some(3), "a new line once someone else spoke");

        // V104-01 A mention of yours: flagged like your name, a pop-up, the Mentions tab.
        let ev = feed(&mut r, "6", "9", "PASTA later?");
        let RoomEvent::Message { message } = &ev[0] else {
            panic!()
        };
        assert!(message.has(Flag::Mention));
        let out = r.take_outcomes();
        let (_, o) = out.last().unwrap();
        assert!(o.notify && !o.sound && o.routes == ["Mentions"]);
    }
}
