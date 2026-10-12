//! Highlights and what chat hides (2.6, F2, F3, F4, F6, C26). Rules are checked in
//! Rust when a message arrives; invalid regexes are kept, so the editor can show them,
//! and simply never match.

mod rules;

pub use rules::{
    Alert, AppRule, BadgeRule, Blocked, BlockedShow, BuiltIn, FilterDef, HiddenUser, IgnoreRule, PhraseRule,
    SentFrom, Similarity, UserRule,
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;
use rules::{MAX_PATTERN_CHARS, MAX_ROUTE_CHARS};

const MAX_RULES: usize = 200;
const MAX_NEVER: usize = 500;
const MAX_FILTERS: usize = 50;
const MAX_FILTER_CHARS: usize = 2000;
const MAX_NAME_CHARS: usize = 40;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Highlights {
    /// Your name, and replies to you.
    pub mention: BuiltIn,
    pub first_message: BuiltIn,
    /// "Highlight my message" with channel points.
    pub redeemed: BuiltIn,
    pub subs: BuiltIn,
    pub announcements: BuiltIn,
    pub watch_streaks: BuiltIn,
    /// Your own messages. Off by default.
    pub own: BuiltIn,
    pub phrases: Vec<PhraseRule>,
    pub users: Vec<UserRule>,
    pub badges: Vec<BadgeRule>,
    /// C26
    pub apps: Vec<AppRule>,
    /// Logins that never highlight or play sounds, e.g. bots.
    pub never: Vec<String>,
    /// C27 Channel logins whose chats never play sounds or flash.
    pub muted: Vec<String>,
    /// C32 Look-alike letters count when phrases and ignores match.
    pub lookalikes: bool,
    /// F2
    pub ignores: Vec<IgnoreRule>,
    pub blocked: Blocked,
    /// C28 Hidden only for you.
    pub hidden_users: Vec<HiddenUser>,
    /// F3
    pub filters: Vec<FilterDef>,
    /// F6
    pub similar: Similarity,
}

impl Default for Highlights {
    fn default() -> Self {
        Self {
            mention: BuiltIn::on(),
            first_message: BuiltIn::on(),
            redeemed: BuiltIn::on(),
            subs: BuiltIn::on(),
            announcements: BuiltIn::on(),
            watch_streaks: BuiltIn::on(),
            own: BuiltIn::off(),
            phrases: Vec::new(),
            users: Vec::new(),
            badges: Vec::new(),
            apps: Vec::new(),
            never: Vec::new(),
            muted: Vec::new(),
            lookalikes: false,
            ignores: Vec::new(),
            blocked: Blocked::default(),
            hidden_users: Vec::new(),
            filters: Vec::new(),
            similar: Similarity::default(),
        }
    }
}

fn clip(s: &mut String, max: usize) {
    if s.chars().count() > max {
        *s = s.chars().take(max).collect();
    }
}

impl Schema for Highlights {
    fn sanitize(&mut self) {
        for b in [
            &mut self.mention,
            &mut self.first_message,
            &mut self.redeemed,
            &mut self.subs,
            &mut self.announcements,
            &mut self.watch_streaks,
            &mut self.own,
        ] {
            b.alert.sanitize();
        }
        self.phrases.retain(|r| !r.pattern.trim().is_empty());
        self.phrases.truncate(MAX_RULES);
        for r in &mut self.phrases {
            clip(&mut r.pattern, MAX_PATTERN_CHARS);
            r.alert.sanitize();
        }
        self.users.retain_mut(|r| {
            r.login = r.login.trim().trim_start_matches('@').to_owned();
            if !r.regex {
                r.login.make_ascii_lowercase();
            }
            clip(&mut r.login, MAX_PATTERN_CHARS);
            r.alert.sanitize();
            !r.login.is_empty()
        });
        self.users.truncate(MAX_RULES);
        self.badges.retain_mut(|r| {
            r.badge = r.badge.trim().to_ascii_lowercase();
            r.alert.sanitize();
            !r.badge.is_empty() && r.badge.len() <= 60
        });
        self.badges.truncate(MAX_RULES);
        for r in &mut self.apps {
            r.alert.sanitize();
        }
        self.apps.truncate(MAX_RULES);
        for list in [&mut self.never, &mut self.muted] {
            for login in list.iter_mut() {
                *login = login.trim().trim_start_matches(['@', '#']).to_ascii_lowercase();
            }
            list.retain(|l| !l.is_empty());
            list.sort_unstable();
            list.dedup();
            list.truncate(MAX_NEVER);
        }
        self.hidden_users.retain(|u| !u.id.is_empty());
        self.hidden_users.dedup_by(|a, b| a.id == b.id);
        self.hidden_users.truncate(MAX_NEVER);
        self.ignores.retain(|r| !r.pattern.trim().is_empty());
        self.ignores.truncate(MAX_RULES);
        for r in &mut self.ignores {
            clip(&mut r.pattern, MAX_PATTERN_CHARS);
            clip(&mut r.replace, MAX_PATTERN_CHARS);
            r.route = r.route.trim().chars().take(MAX_ROUTE_CHARS).collect();
        }
        self.filters.retain_mut(|f| {
            clip(&mut f.name, MAX_NAME_CHARS);
            clip(&mut f.expr, MAX_FILTER_CHARS);
            !f.id.is_empty()
        });
        self.filters.dedup_by(|a, b| a.id == b.id);
        self.filters.truncate(MAX_FILTERS);
        let s = &mut self.similar;
        s.threshold = s.threshold.clamp(50, 100);
        s.max_age_secs = s.max_age_secs.clamp(1, 600);
        s.depth = s.depth.clamp(1, 5);
    }
}

impl Highlights {
    /// F4 Every feed tab name the rules send messages to, A-Z.
    pub fn routes(&self) -> Vec<String> {
        let builtins = [
            &self.mention,
            &self.first_message,
            &self.redeemed,
            &self.subs,
            &self.announcements,
            &self.watch_streaks,
            &self.own,
        ];
        let mut names: Vec<String> = builtins
            .iter()
            .map(|b| &b.alert.route)
            .chain(self.phrases.iter().map(|r| &r.alert.route))
            .chain(self.users.iter().map(|r| &r.alert.route))
            .chain(self.badges.iter().map(|r| &r.alert.route))
            .chain(self.apps.iter().map(|r| &r.alert.route))
            .chain(self.ignores.iter().map(|r| &r.route))
            .filter(|r| !r.is_empty())
            .cloned()
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_cleans_rules() {
        let mut h = Highlights {
            phrases: vec![
                PhraseRule {
                    pattern: "  ".into(),
                    ..PhraseRule::default()
                },
                PhraseRule {
                    pattern: "hi".into(),
                    alert: Alert {
                        color: "#FF0000".into(),
                        route: "  Pings ".into(),
                        ..Alert::default()
                    },
                    ..PhraseRule::default()
                },
            ],
            users: vec![UserRule {
                login: " @Bob ".into(),
                ..UserRule::default()
            }],
            never: vec!["Nightbot".into(), "nightbot".into(), "".into()],
            ignores: vec![IgnoreRule {
                pattern: "x".into(),
                route: "spam".into(),
                ..IgnoreRule::default()
            }],
            ..Highlights::default()
        };
        h.mention.alert.color = "red".into();
        h.similar.threshold = 10;
        h.sanitize();
        assert_eq!(h.phrases.len(), 1);
        assert_eq!(h.phrases[0].alert.color, "#ff0000");
        assert_eq!(h.mention.alert.color, "");
        assert_eq!(h.users[0].login, "bob");
        assert_eq!(h.never, ["nightbot"]);
        assert_eq!(h.similar.threshold, 50);
        assert_eq!(h.routes(), ["Pings", "spam"]);
    }
}
