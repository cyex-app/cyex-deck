//! What the user's rules do to a message as it arrives (2.6, F2, F4, F6, C2): hide it,
//! swap words, highlight it, flag near-duplicates, count repeats, and name the feed tabs
//! it's copied into. Built once from settings; every room shares it.

mod compile;
mod lookalike;

use std::collections::HashSet;

use cyex_settings::schema::{Alert, BlockedShow, Highlights};

use crate::model::{ChatMessage, Client, Flag, Fragment, Highlight, HighlightKind, NoticeKind};
use crate::similar::{RepeatCheck, SimilarCheck};
use compile::Rule;

/// What happened to a message, for the app: sounds, the taskbar, feed tabs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub sound: bool,
    pub flash: bool,
    /// F4 Feed tabs to copy it into.
    pub routes: Vec<String>,
}

#[derive(Default)]
pub struct Rules {
    builtin: Vec<(HighlightKind, Alert)>,
    phrases: Vec<Rule>,
    users: Vec<Rule>,
    badges: Vec<(String, Option<String>, Alert)>,
    apps: Vec<(Client, Alert)>,
    never: HashSet<String>,
    /// C27 Channels that never ping.
    muted: HashSet<String>,
    /// C28
    hidden_users: HashSet<String>,
    /// C32
    lookalikes: bool,
    ignores: Vec<compile::Ignore>,
    blocked: Option<BlockedShow>,
    /// F2 Twitch blocks of every signed-in account, by user id.
    blocked_ids: HashSet<String>,
    pub similar: Option<SimilarCheck>,
    pub similar_own: bool,
    pub repeat: Option<RepeatCheck>,
}

impl Rules {
    pub fn new(h: &Highlights) -> Self {
        let builtin = [
            (HighlightKind::Mention, &h.mention),
            (HighlightKind::FirstMessage, &h.first_message),
            (HighlightKind::Redeemed, &h.redeemed),
            (HighlightKind::Sub, &h.subs),
            (HighlightKind::Announcement, &h.announcements),
            (HighlightKind::WatchStreak, &h.watch_streaks),
            (HighlightKind::Own, &h.own),
        ]
        .into_iter()
        .filter(|(_, b)| b.enabled)
        .map(|(k, b)| (k, b.alert.clone()))
        .collect();
        let s = &h.similar;
        Self {
            builtin,
            phrases: compile::phrases(&h.phrases),
            users: compile::users(&h.users),
            badges: h
                .badges
                .iter()
                .filter(|b| b.enabled)
                .map(|b| match b.badge.split_once('/') {
                    Some((set, v)) => (set.to_owned(), Some(v.to_owned()), b.alert.clone()),
                    None => (b.badge.clone(), None, b.alert.clone()),
                })
                .collect(),
            apps: h
                .apps
                .iter()
                .filter(|a| a.enabled)
                .map(|a| (compile::client(a.app), a.alert.clone()))
                .collect(),
            never: h.never.iter().cloned().collect(),
            muted: h.muted.iter().cloned().collect(),
            hidden_users: h.hidden_users.iter().map(|u| u.id.clone()).collect(),
            lookalikes: h.lookalikes,
            ignores: compile::ignores(&h.ignores),
            blocked: h.blocked.enabled.then_some(h.blocked.show),
            blocked_ids: HashSet::new(),
            similar: s.enabled.then(|| SimilarCheck {
                threshold: f32::from(s.threshold) / 100.0,
                max_age_ms: i64::from(s.max_age_secs) * 1000,
                depth: usize::from(s.depth),
                same_user: s.same_user,
            }),
            similar_own: s.own,
            repeat: None,
        }
    }

    pub fn set_blocked(&mut self, ids: HashSet<String>) {
        self.blocked_ids = ids;
    }

    /// C27 This chat never plays sounds or flashes.
    pub fn muted(&self, channel: &str) -> bool {
        self.muted.contains(channel)
    }

    /// The text rules match against: C32 folds look-alike letters.
    fn text_of(&self, m: &ChatMessage) -> String {
        let text = crate::model::plain(&m.body);
        if self.lookalikes {
            lookalike::fold(&text)
        } else {
            text
        }
    }

    /// F2 Hidden from this room, given our role in it. Copies still go to `routes`.
    pub fn hidden(&self, m: &ChatMessage, role: Role, routes: &mut Vec<String>) -> bool {
        if m.sender
            .as_ref()
            .is_some_and(|s| self.hidden_users.contains(&s.id))
        {
            return true;
        }
        if let (Some(show), Some(s)) = (self.blocked, &m.sender)
            && self.blocked_ids.contains(&s.id)
        {
            let shown = match show {
                BlockedShow::Never => false,
                BlockedShow::Moderator => role != Role::Viewer,
                BlockedShow::Broadcaster => role == Role::Broadcaster,
            };
            if !shown {
                return true;
            }
        }
        let text = self.text_of(m);
        let mut hide = false;
        for ig in self
            .ignores
            .iter()
            .filter(|i| i.block && i.matcher.is_match(&text))
        {
            hide = true;
            if !ig.route.is_empty() {
                routes.push(ig.route.clone());
            }
        }
        hide
    }

    /// F2 Swaps ignored words in the text parts of a message.
    pub fn replace(&self, m: &mut ChatMessage) {
        for ig in self.ignores.iter().filter(|i| !i.block) {
            for f in &mut m.body {
                if let Fragment::Text { text } = f
                    && let Some(new) = ig.matcher.replace(text, &ig.replace)
                {
                    *text = new;
                }
            }
        }
    }

    /// 2.6 Picks the highlight; the first rule to match sets the color (Chatterino's
    /// order), and every match adds its sound, flash and feed tabs.
    pub fn highlight(&self, m: &mut ChatMessage) -> Outcome {
        let mut out = Outcome::default();
        let mut hit: Option<Highlight> = None;
        let mut take = |kind: HighlightKind, a: &Alert| {
            if hit.is_none() {
                hit = Some(Highlight {
                    kind,
                    color: (!a.color.is_empty()).then(|| a.color.clone()),
                });
            }
            out.sound |= a.sound;
            out.flash |= a.flash;
            if !a.route.is_empty() && !out.routes.contains(&a.route) {
                out.routes.push(a.route.clone());
            }
        };
        let builtin = |k: HighlightKind| self.builtin.iter().find(|(b, _)| *b == k).map(|(_, a)| a);
        if let Some(sender) = m.sender.as_ref() {
            if self.never.contains(&sender.login) {
                return Outcome::default();
            }
            let own = m.has(Flag::Own);
            if m.has(Flag::Mention)
                && let Some(a) = builtin(HighlightKind::Mention)
            {
                take(HighlightKind::Mention, a);
            }
            if !own {
                let text = self.text_of(m);
                for r in self.phrases.iter().filter(|r| r.matcher.is_match(&text)) {
                    take(HighlightKind::Rule, &r.alert);
                }
            }
            if own && let Some(a) = builtin(HighlightKind::Own) {
                take(HighlightKind::Own, a);
            }
            for r in self.users.iter().filter(|r| r.matcher.is_match(&sender.login)) {
                take(HighlightKind::Rule, &r.alert);
            }
            for (set, version, a) in &self.badges {
                if sender
                    .badges
                    .iter()
                    .any(|b| b.set == *set && version.as_ref().is_none_or(|v| *v == b.version))
                {
                    take(HighlightKind::Rule, a);
                }
            }
            if let Some(client) = m.meta.client {
                for (_, a) in self.apps.iter().filter(|(c, _)| *c == client) {
                    take(HighlightKind::Rule, a);
                }
            }
            if m.has(Flag::Highlighted)
                && let Some(a) = builtin(HighlightKind::Redeemed)
            {
                take(HighlightKind::Redeemed, a);
            }
            if m.has(Flag::FirstMessage)
                && let Some(a) = builtin(HighlightKind::FirstMessage)
            {
                take(HighlightKind::FirstMessage, a);
            }
        }
        let notice = match m.notice.as_ref().map(|n| n.kind) {
            Some(NoticeKind::Sub | NoticeKind::Gift) => Some(HighlightKind::Sub),
            Some(NoticeKind::Announcement) => Some(HighlightKind::Announcement),
            Some(NoticeKind::Milestone) => Some(HighlightKind::WatchStreak),
            _ => None,
        };
        if let Some(kind) = notice
            && let Some(a) = builtin(kind)
        {
            take(kind, a);
        }
        if hit.is_some() {
            m.meta.highlight = hit;
        }
        out
    }
}

/// Our role in a room, for blocked users (F2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Viewer,
    Moderator,
    Broadcaster,
}
