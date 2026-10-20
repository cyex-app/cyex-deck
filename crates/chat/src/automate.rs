//! S4 Chat automations: when someone says a trigger in a chat, the rule's response runs
//! as if you typed it. Rules have cooldowns per chat and per person, and every chat has a
//! rate limit (moltorino's: at least 1.5 s apart, at most N per 30 s). Your own lines,
//! history and system lines never set a rule off.

use std::collections::{HashMap, VecDeque};

use cyex_settings::schema::{Access, Automation, Commands, TriggerMatch};
use regex::{Regex, RegexBuilder};

use crate::model::{ChatMessage, Flag, ModRole, plain};

const GAP_MS: i64 = 1500;
const WINDOW_MS: i64 = 30_000;
/// V107-R1 Per-person cooldowns a chat holds before the run-out ones are swept.
const MAX_USER_COOLDOWNS: usize = 512;

/// A rule that went off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fired {
    /// Index in the settings list.
    pub rule: usize,
    pub response: String,
    /// What followed the trigger, for `$1-`.
    pub args: String,
    /// Message id to reply to, when the rule replies.
    pub reply_to: Option<String>,
    /// Including this time.
    pub count: u64,
}

struct Rule {
    cfg: Automation,
    regex: Option<Regex>,
}

#[derive(Default)]
struct Chat {
    sent: VecDeque<i64>,
    rule_at: HashMap<usize, i64>,
    user_at: HashMap<(usize, String), i64>,
}

#[derive(Default)]
pub struct Automations {
    rules: Vec<Rule>,
    limit: usize,
    chats: HashMap<String, Chat>,
}

impl Automations {
    pub fn new(c: &Commands) -> Self {
        let rules = if c.automations_on {
            c.automations
                .iter()
                .map(|a| Rule {
                    regex: (a.matching == TriggerMatch::Regex)
                        .then(|| RegexBuilder::new(&a.trigger).case_insensitive(true).build().ok())
                        .flatten(),
                    cfg: a.clone(),
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            rules,
            limit: usize::from(c.automation_limit.max(1)),
            chats: HashMap::new(),
        }
    }

    /// New rules; cooldowns and rate limits carry over for chats some rule is still for.
    pub fn update(&mut self, c: &Commands) {
        let mut chats = std::mem::take(&mut self.chats);
        *self = Self::new(c);
        chats.retain(|channel, _| self.rules.iter().any(|r| r.cfg.channels.iter().any(|c| c == channel)));
        self.chats = chats;
    }

    pub fn is_empty(&self) -> bool {
        self.rules.iter().all(|r| !r.cfg.enabled)
    }

    /// The first rule a message sets off in `channel`, if any may run now.
    pub fn check(&mut self, channel: &str, m: &ChatMessage, live: bool, now_ms: i64) -> Option<Fired> {
        if m.has(Flag::Own) || m.has(Flag::History) || m.has(Flag::Deleted) || m.notice.is_some() {
            return None;
        }
        let sender = m.sender.as_ref()?;
        // V107-R1 No state for chats no rule is for.
        if !self.rules.iter().any(|r| r.cfg.enabled && r.cfg.channels.iter().any(|c| c == channel)) {
            return None;
        }
        let text = plain(&m.body);
        let text = text.trim();
        let chat = self.chats.entry(channel.to_owned()).or_default();
        while chat.sent.front().is_some_and(|t| now_ms - t > WINDOW_MS) {
            chat.sent.pop_front();
        }
        if chat.user_at.len() >= MAX_USER_COOLDOWNS {
            let rules = &self.rules;
            chat.user_at.retain(|(i, _), t| {
                rules
                    .get(*i)
                    .is_some_and(|r| now_ms - *t < i64::from(r.cfg.user_cooldown_secs) * 1000)
            });
        }
        // Still full of live cooldowns (long ones in a busy chat): the oldest half goes.
        if chat.user_at.len() >= MAX_USER_COOLDOWNS {
            let mut by_age: Vec<_> = chat.user_at.drain().collect();
            by_age.sort_unstable_by_key(|(_, t)| *t);
            let newest = by_age.split_off(by_age.len() - MAX_USER_COOLDOWNS / 2);
            chat.user_at = newest.into_iter().collect();
        }
        for (i, rule) in self.rules.iter_mut().enumerate() {
            let a = &rule.cfg;
            if !a.enabled || !a.channels.iter().any(|c| c == channel) || (a.live_only && !live) {
                continue;
            }
            let Some(args) = matches(a, rule.regex.as_ref(), text) else {
                continue;
            };
            if !allowed(a.who, m) {
                continue;
            }
            // A matching rule on cooldown stops the search, so a later one doesn't answer instead.
            let rule_cool = chat
                .rule_at
                .get(&i)
                .is_some_and(|t| now_ms - t < i64::from(a.cooldown_secs) * 1000);
            let user_key = (i, sender.id.clone());
            let user_cool = chat
                .user_at
                .get(&user_key)
                .is_some_and(|t| now_ms - t < i64::from(a.user_cooldown_secs) * 1000);
            let busy = chat.sent.len() >= self.limit || chat.sent.back().is_some_and(|t| now_ms - t < GAP_MS);
            if rule_cool || user_cool || busy {
                return None;
            }
            chat.rule_at.insert(i, now_ms);
            chat.user_at.insert(user_key, now_ms);
            chat.sent.push_back(now_ms);
            rule.cfg.count += 1;
            return Some(Fired {
                rule: i,
                response: rule.cfg.response.clone(),
                args,
                reply_to: rule.cfg.reply.then(|| m.id.clone()),
                count: rule.cfg.count,
            });
        }
        None
    }
}

/// What followed the trigger, when the message matches.
fn matches(a: &Automation, regex: Option<&Regex>, text: &str) -> Option<String> {
    let trigger = a.trigger.trim();
    if trigger.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    let t = trigger.to_lowercase();
    match a.matching {
        TriggerMatch::Command => {
            let (first, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
            first
                .eq_ignore_ascii_case(trigger)
                .then(|| rest.trim().to_owned())
        }
        TriggerMatch::Exact => (lower == t).then(String::new),
        TriggerMatch::StartsWith => lower
            .starts_with(&t)
            .then(|| text[trigger.len().min(text.len())..].trim().to_owned()),
        TriggerMatch::Contains => lower.contains(&t).then(|| text.to_owned()),
        TriggerMatch::Regex => regex?.is_match(text).then(|| text.to_owned()),
    }
}

fn allowed(who: Access, m: &ChatMessage) -> bool {
    let Some(s) = m.sender.as_ref() else { return false };
    let has = |set: &str| s.badges.iter().any(|b| b.set == set);
    if ModRole::of_badges(&s.badges).is_some() {
        return true;
    }
    match who {
        Access::Everyone => true,
        Access::Subscribers => has("subscriber") || has("founder") || has("vip"),
        Access::Vips => has("vip"),
        Access::Moderators | Access::Broadcaster => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::{Ctx, Incoming, build};
    use crate::irc::Line;

    fn msg(badges: &str, text: &str) -> ChatMessage {
        let raw = format!("@badges={badges};id=m;user-id=7 :u!u@u PRIVMSG #chan :{text}");
        let mut colors = Default::default();
        let mut ctx = Ctx::new(None, &mut colors, 0);
        match build(&Line::parse(&raw).unwrap(), &mut ctx) {
            Some(Incoming::Message(m)) => m,
            _ => panic!(),
        }
    }

    fn rules(list: Vec<Automation>) -> Automations {
        Automations::new(&Commands {
            automations_on: true,
            automations: list,
            automation_limit: 2,
            ..Commands::default()
        })
    }

    #[test]
    fn triggers_cooldowns_and_limits() {
        let mut a = rules(vec![
            Automation {
                channels: vec!["chan".into()],
                trigger: "!discord".into(),
                response: "discord.gg/x $1-".into(),
                reply: true,
                ..Automation::default()
            },
            Automation {
                channels: vec!["chan".into()],
                trigger: "hello".into(),
                matching: TriggerMatch::Contains,
                who: Access::Subscribers,
                cooldown_secs: 0,
                ..Automation::default()
            },
        ]);
        let f = a.check("chan", &msg("", "!Discord please"), false, 0).unwrap();
        assert_eq!(
            (f.rule, f.args.as_str(), f.reply_to.as_deref(), f.count),
            (0, "please", Some("m"), 1)
        );
        assert!(
            a.check("chan", &msg("", "!discord"), false, 5_000).is_none(),
            "rule cooldown"
        );
        assert!(
            a.check("chan", &msg("", "oh hello"), false, 5_000).is_none(),
            "not a sub"
        );
        assert!(
            a.check("chan", &msg("subscriber/1", "oh hello"), false, 5_500)
                .is_some()
        );
        assert!(
            a.check("chan", &msg("moderator/1", "hello"), false, 6_000)
                .is_none(),
            "1.5 s apart"
        );
        assert!(
            a.check("chan", &msg("moderator/1", "hello"), false, 8_000)
                .is_none(),
            "2 per 30 s"
        );
        assert!(
            a.check("other", &msg("", "!discord"), false, 31_000).is_none(),
            "not in this chat"
        );
        assert!(
            a.check("chan", &msg("moderator/1", "hello"), false, 31_000)
                .is_some()
        );
    }

    #[test]
    fn v107_r1_cooldowns_stay_bounded_and_other_chats_keep_nothing() {
        let mut a = Automations::new(&Commands {
            automations_on: true,
            automations: vec![Automation {
                channels: vec!["chan".into()],
                trigger: "hi".into(),
                cooldown_secs: 0,
                user_cooldown_secs: 60,
                ..Automation::default()
            }],
            automation_limit: 20,
            ..Commands::default()
        });
        let mut fired = 0;
        for i in 0..5_000i64 {
            let raw = format!("@id=m{i};user-id={i} :u{i}!u@u PRIVMSG #chan :hi");
            let mut colors = Default::default();
            let mut ctx = Ctx::new(None, &mut colors, 0);
            let Some(Incoming::Message(m)) = build(&Line::parse(&raw).unwrap(), &mut ctx) else {
                panic!()
            };
            fired += usize::from(a.check("chan", &m, false, i * 2_000).is_some());
            assert!(a.check(&format!("other{i}"), &m, false, i * 2_000).is_none());
        }
        assert_eq!(fired, 5_000);
        assert_eq!(a.chats.len(), 1);
        assert!(a.chats["chan"].user_at.len() <= MAX_USER_COOLDOWNS);
    }

    fn user_msg(i: i64) -> ChatMessage {
        let raw = format!("@id=m{i};user-id={i} :u{i}!u@u PRIVMSG #chan :hi");
        let mut colors = Default::default();
        let mut ctx = Ctx::new(None, &mut colors, 0);
        let Some(Incoming::Message(m)) = build(&Line::parse(&raw).unwrap(), &mut ctx) else { panic!() };
        m
    }

    #[test]
    fn v107_long_cooldowns_stay_capped_and_old_chats_go() {
        let rule = |channel: &str| Automation {
            channels: vec![channel.into()],
            trigger: "hi".into(),
            cooldown_secs: 0,
            user_cooldown_secs: 86_400,
            ..Automation::default()
        };
        let cmds = |list| Commands { automations_on: true, automations: list, automation_limit: 20, ..Commands::default() };
        let mut a = Automations::new(&cmds(vec![rule("chan")]));
        // A day-long per-person cooldown never runs out here: the cap still holds.
        for i in 0..3_000i64 {
            a.check("chan", &user_msg(i), false, i * 2_000);
        }
        let n = a.chats["chan"].user_at.len();
        assert!(n <= MAX_USER_COOLDOWNS, "{n}");
        assert!(a.chats["chan"].user_at.contains_key(&(0, "2999".into())), "the newest stay");
        // The rule moves to another chat: the old chat's state goes.
        a.update(&cmds(vec![rule("other")]));
        assert!(a.chats.is_empty());
    }
}
