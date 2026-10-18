//! What the user's rules do to a message as it arrives (2.6, F2, F4, F6, C2): hide it,
//! swap words, highlight it, flag near-duplicates, count repeats, and name the feed tabs
//! it's copied into. Your name and replies ping first, then words (V106-33: old mentions
//! and phrases in one list). Built once from settings; every room shares it.

mod compile;
pub mod pcre;

use std::collections::HashSet;

use cyex_settings::schema::{Alert, BlockedShow, Highlights};

use crate::lookalike;
use crate::mentions::{self, MentionSet};
use crate::model::{ChatMessage, Client, Flag, Fragment, Highlight, HighlightKind, NoticeKind, plain};
use crate::similar::{RepeatCheck, SimilarCheck};
use compile::Rule;

/// V105-16 A Chatterino (PCRE2) highlight pattern in the Rust flavor, ready to store, or
/// why it can't be imported.
pub fn import_regex(pattern: &str) -> Result<String, String> {
    let rust = pcre::to_rust(pattern)?;
    match compile::highlight_error(&rust) {
        Some(e) => Err(e),
        None => Ok(rust),
    }
}

/// What happened to a message, for the app: sounds, the taskbar, pop-ups, feed tabs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub sound: bool,
    pub flash: bool,
    /// V104-01 A Windows pop-up.
    pub notify: bool,
    /// V104-01 The first match's own sound file, played instead of the chime.
    pub sound_file: Option<String>,
    /// F4 Feed tabs to copy it into.
    pub routes: Vec<String>,
}

impl Outcome {
    /// V106-33 What every match adds: sound (its own file wins if it's the first), flash,
    /// pop-up, the Mentions tab (`show`), its feed tab.
    pub fn add(&mut self, a: &Alert) {
        self.sound |= a.sound;
        self.flash |= a.flash;
        self.notify |= a.notify;
        if a.sound && !a.sound_file.is_empty() && self.sound_file.is_none() {
            self.sound_file = Some(a.sound_file.clone());
        }
        if a.show {
            self.feed();
        }
        if !a.route.is_empty() && !self.has_route(&a.route) {
            self.routes.push(a.route.clone());
        }
    }

    /// The Mentions feed tab, once.
    fn feed(&mut self) {
        if !self.has_route(mentions::FEED) {
            self.routes.push(mentions::FEED.to_owned());
        }
    }

    /// R22 One test for both: "Mentions" and "mentions" are the same tab.
    fn has_route(&self, r: &str) -> bool {
        self.routes.iter().any(|x| x.trim().eq_ignore_ascii_case(r.trim()))
    }

    pub fn pings(&self) -> bool {
        self.sound || self.flash || self.notify
    }
}

#[derive(Default)]
pub struct Rules {
    builtin: Vec<(HighlightKind, Alert)>,
    /// V106-33 Your name and replies to you, when that row is on.
    me: Option<Alert>,
    replies: Option<Alert>,
    /// V106-33 Old mentions and phrases, in list order.
    words: MentionSet,
    users: Vec<Rule>,
    badges: Vec<(String, compile::BadgeMatch, Alert)>,
    apps: Vec<(Client, Alert)>,
    never: HashSet<String>,
    /// C27 Channels that never ping.
    muted: HashSet<String>,
    /// C28
    hidden_users: HashSet<String>,
    /// C32 Ignores only (V106-33: words have their own).
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
            me: h.me.enabled.then(|| h.me.alert.clone()),
            replies: h.replies.enabled.then(|| h.replies.alert.clone()),
            words: MentionSet::new(&h.phrases),
            users: compile::users(&h.users),
            badges: h
                .badges
                .iter()
                .filter(|b| b.enabled && !b.is_blank())
                .map(|b| {
                    let (set, want) = compile::badge(&b.badge);
                    (set, want, b.alert.clone())
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

    /// The text ignores match against: C32 folds look-alike letters.
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
    /// order: your name, replies, words), and every match adds its sound, flash and feed tabs.
    /// V106-33 `Flag::Mention` is settled here: your name or a reply only while that row is
    /// on (O2: off is fully off), or a word with the Mentions tab on (Q10).
    pub fn highlight(&self, m: &mut ChatMessage) -> Outcome {
        let mut pick = Pick::default();
        let builtin = |k: HighlightKind| self.builtin.iter().find(|(b, _)| *b == k).map(|(_, a)| a);
        let own = m.has(Flag::Own);
        let me = self.me.as_ref().filter(|_| !own && m.has(Flag::NamesMe));
        let reply = self.replies.as_ref().filter(|_| !own && m.has(Flag::RepliesToMe));
        let mut mentioned = me.is_some() || reply.is_some();
        if m.has(Flag::Mention) {
            m.flags.retain(|f| *f != Flag::Mention);
        }
        if let Some(sender) = m.sender.as_ref() {
            if self.never.contains(&sender.login) {
                // Never highlights, but your name still lights up the tab.
                if mentioned {
                    m.set(Flag::Mention);
                }
                return Outcome::default();
            }
            // Q4 Both always go to the Mentions tab (sanitize locks `show` on too).
            for a in me.into_iter().chain(reply) {
                pick.mention(a);
                pick.out.feed();
            }
            if !own && !self.words.is_empty() {
                self.words.each_match(&plain(&m.body), &mut String::new(), |r| {
                    if r.alert.show {
                        pick.mention(&r.alert);
                        mentioned = true;
                    } else {
                        pick.take(HighlightKind::Rule, &r.alert);
                    }
                });
            }
            if own && let Some(a) = builtin(HighlightKind::Own) {
                pick.take(HighlightKind::Own, a);
            }
            for r in self.users.iter().filter(|r| r.matcher.is_match(&sender.login)) {
                pick.take(HighlightKind::Rule, &r.alert);
            }
            for (set, want, a) in &self.badges {
                if sender
                    .badges
                    .iter()
                    .any(|b| b.set == *set && want.matches(&b.version))
                {
                    pick.take(HighlightKind::Rule, a);
                }
            }
            if let Some(client) = m.meta.client {
                for (_, a) in self.apps.iter().filter(|(c, _)| *c == client) {
                    pick.take(HighlightKind::Rule, a);
                }
            }
            if m.has(Flag::Highlighted)
                && let Some(a) = builtin(HighlightKind::Redeemed)
            {
                pick.take(HighlightKind::Redeemed, a);
            }
            if m.has(Flag::FirstMessage)
                && let Some(a) = builtin(HighlightKind::FirstMessage)
            {
                pick.take(HighlightKind::FirstMessage, a);
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
            pick.take(kind, a);
        }
        // Q10 Counts as a mention: the tab lights up, filters and search see it.
        if mentioned {
            m.set(Flag::Mention);
        }
        if pick.hit.is_some() {
            m.meta.highlight = pick.hit;
        }
        pick.out
    }
}

/// The highlight so far and what every match asked for.
#[derive(Default)]
struct Pick {
    out: Outcome,
    hit: Option<Highlight>,
}

impl Pick {
    fn color(&mut self, kind: HighlightKind, color: &str) {
        if self.hit.is_none() {
            self.hit = Some(Highlight {
                kind,
                color: (!color.is_empty()).then(|| color.to_owned()),
            });
        }
    }

    fn take(&mut self, kind: HighlightKind, a: &Alert) {
        self.color(kind, &a.color);
        self.out.add(a);
    }

    fn mention(&mut self, a: &Alert) {
        self.take(HighlightKind::Mention, a);
    }
}

/// Our role in a room, for blocked users (F2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Viewer,
    Moderator,
    Broadcaster,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_dedup_ignoring_case() {
        let mut o = Outcome::default();
        o.add(&Alert { route: "mentions".into(), ..Alert::default() });
        o.add(&Alert { show: true, ..Alert::default() });
        o.add(&Alert { route: "MENTIONS".into(), show: true, ..Alert::default() });
        assert_eq!(o.routes, ["mentions"]);
    }
}
