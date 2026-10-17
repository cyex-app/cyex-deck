//! Highlights and what chat hides (2.6, F2, F3, F4, F6, C26). Rules are checked in
//! Rust when a message arrives; invalid regexes are kept, so the editor can show them,
//! and simply never match.

mod rules;

pub use rules::{
    Alert, AppRule, BadgeRule, Blocked, BlockedShow, BuiltIn, FilterDef, HiddenUser, IgnoreRule, PhraseRule,
    RegexFlavor, SOUND_TYPES, SentFrom, Similarity, UserRule, is_sound_file,
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;
use rules::{MAX_PATTERN_CHARS, MAX_ROUTE_CHARS};

pub(crate) const MAX_RULES: usize = 200;
/// V106-33 200 phrases + 100 mentions: a full merge loses nothing.
pub(crate) const MAX_WORDS: usize = 300;
/// V104-02
pub(crate) const MAX_GLOBAL: usize = 100;
const MAX_NEVER: usize = 500;
const MAX_FILTERS: usize = 50;
const MAX_FILTER_CHARS: usize = 2000;
const MAX_NAME_CHARS: usize = 40;
const MAX_BADGE_CHARS: usize = 60;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Highlights {
    /// V106-33 Your name (was `mention`, then Mentions' `me`). V106-09 sound on.
    pub me: BuiltIn,
    /// V106-33 Replies to your messages.
    pub replies: BuiltIn,
    pub first_message: BuiltIn,
    /// "Highlight my message" with channel points.
    pub redeemed: BuiltIn,
    pub subs: BuiltIn,
    pub announcements: BuiltIn,
    pub watch_streaks: BuiltIn,
    /// Your own messages. Off by default.
    pub own: BuiltIn,
    /// V104-01 + 2.6 Words or regexes: old mentions and highlight phrases in one list. Order is precedence.
    pub phrases: Vec<PhraseRule>,
    pub users: Vec<UserRule>,
    pub badges: Vec<BadgeRule>,
    /// C26
    pub apps: Vec<AppRule>,
    /// Logins that never highlight or play sounds, e.g. bots.
    pub never: Vec<String>,
    /// C27 Channel logins whose chats never play sounds or flash.
    pub muted: Vec<String>,
    /// C32 Look-alike letters in ignores (V106-33: words have their own).
    pub lookalikes: bool,
    /// V104-02 Global mentions from logs.cyex.app.
    pub global: bool,
    pub global_list: Vec<PhraseRule>,
    /// F2
    pub ignores: Vec<IgnoreRule>,
    pub blocked: Blocked,
    /// C28 Hidden only for you.
    pub hidden_users: Vec<HiddenUser>,
    /// F3
    pub filters: Vec<FilterDef>,
    /// F6
    pub similar: Similarity,
    /// V106-33 Written by v1.06+: an old file (no key) still has to be merged. Always true after sanitize.
    pub mentions_merged: bool,
    /// V106-33 The duplicates dialog was answered. The merge sets false when it found duplicates.
    pub duplicates_asked: bool,
}

impl Default for Highlights {
    fn default() -> Self {
        Self {
            me: BuiltIn::you(true),
            replies: BuiltIn::you(false),
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
            global: false,
            global_list: Vec::new(),
            ignores: Vec::new(),
            blocked: Blocked::default(),
            hidden_users: Vec::new(),
            filters: Vec::new(),
            similar: Similarity::default(),
            mentions_merged: true,
            duplicates_asked: true,
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
        for b in self.built_ins_mut() {
            b.alert.sanitize();
        }
        // V106-33 Q4 Your name and replies always go to the Mentions tab.
        self.me.alert.show = true;
        self.replies.alert.show = true;
        // V105-12 Saving keeps empty rules: one was just added and is being filled in. prune_blank drops them once the page is left.
        self.phrases.truncate(MAX_WORDS);
        self.global_list.truncate(MAX_GLOBAL);
        for r in self.phrases.iter_mut().chain(&mut self.global_list) {
            clip(&mut r.pattern, MAX_PATTERN_CHARS);
            r.alert.sanitize();
        }
        self.mentions_merged = true;
        for r in &mut self.users {
            r.login = r.login.trim().trim_start_matches('@').to_owned();
            if !r.regex {
                r.login.make_ascii_lowercase();
            }
            clip(&mut r.login, MAX_PATTERN_CHARS);
            r.alert.sanitize();
        }
        self.users.truncate(MAX_RULES);
        for r in &mut self.badges {
            r.badge = r.badge.trim().to_ascii_lowercase();
            clip(&mut r.badge, MAX_BADGE_CHARS);
            r.alert.sanitize();
        }
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
    /// V105-12 Drops the rules left empty. True when something went.
    pub fn prune_blank(&mut self) -> bool {
        let count = |h: &Self| {
            (h.phrases.len(), h.global_list.len(), h.users.len(), h.badges.len(), h.ignores.len(), h.filters.len())
        };
        let before = count(self);
        self.phrases.retain(|r| !r.is_blank());
        self.global_list.retain(|r| !r.is_blank());
        self.users.retain(|r| !r.is_blank());
        self.badges.retain(|r| !r.is_blank());
        self.ignores.retain(|r| !r.is_blank());
        self.filters.retain(|f| !f.is_blank());
        before != count(self)
    }

    fn built_ins_mut(&mut self) -> [&mut BuiltIn; 8] {
        [
            &mut self.me,
            &mut self.replies,
            &mut self.first_message,
            &mut self.redeemed,
            &mut self.subs,
            &mut self.announcements,
            &mut self.watch_streaks,
            &mut self.own,
        ]
    }

    /// V106-33 Every alert: the 8 built-ins, words, people, badges, apps and global mentions.
    pub fn alerts(&self) -> impl Iterator<Item = &Alert> {
        [
            &self.me,
            &self.replies,
            &self.first_message,
            &self.redeemed,
            &self.subs,
            &self.announcements,
            &self.watch_streaks,
            &self.own,
        ]
        .into_iter()
        .map(|b| &b.alert)
        .chain(self.phrases.iter().map(|r| &r.alert))
        .chain(self.users.iter().map(|r| &r.alert))
        .chain(self.badges.iter().map(|r| &r.alert))
        .chain(self.apps.iter().map(|r| &r.alert))
        .chain(self.global_list.iter().map(|r| &r.alert))
    }

    /// F4 Every feed tab name the rules send messages to, A-Z.
    pub fn routes(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .alerts()
            .map(|a| &a.route)
            .chain(self.ignores.iter().map(|r| &r.route))
            .filter(|r| !r.is_empty())
            .cloned()
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        names
    }

    /// V104-02 Global mentions to look for: on, with something to find.
    pub fn global_active(&self) -> bool {
        self.global
            && self
                .global_list
                .iter()
                .any(|m| m.enabled && !m.pattern.trim().is_empty())
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
        h.me.alert.color = "red".into();
        h.similar.threshold = 10;
        h.sanitize();
        assert_eq!(h.phrases.len(), 2);
        assert_eq!(h.phrases[1].alert.color, "#ff0000");
        assert_eq!(h.me.alert.color, "");
        assert_eq!(h.users[0].login, "bob");
        assert_eq!(h.never, ["nightbot"]);
        assert_eq!(h.similar.threshold, 50);
        assert_eq!(h.routes(), ["Pings", "spam"]);
    }

    #[test]
    fn sanitize_keeps_empty_rules() {
        let mut h = Highlights {
            phrases: vec![PhraseRule::default()],
            users: vec![UserRule::default()],
            badges: vec![BadgeRule {
                badge: "x".repeat(70),
                ..BadgeRule::default()
            }],
            ignores: vec![IgnoreRule::default()],
            apps: vec![AppRule::default()],
            filters: vec![FilterDef {
                id: "f".into(),
                ..FilterDef::default()
            }],
            ..Highlights::default()
        };
        h.sanitize();
        assert_eq!(h.phrases.len(), 1);
        assert_eq!(h.users.len(), 1);
        assert_eq!(h.badges.len(), 1);
        assert_eq!(h.badges[0].badge.len(), 60);
        assert_eq!(h.ignores.len(), 1);
        assert_eq!(h.apps.len(), 1);
        assert_eq!(h.filters.len(), 1);
    }

    #[test]
    fn prune_blank_drops_only_empty_rules() {
        let mut h = Highlights {
            phrases: vec![
                PhraseRule::default(),
                PhraseRule {
                    pattern: "hi".into(),
                    ..PhraseRule::default()
                },
            ],
            users: vec![
                UserRule::default(),
                UserRule {
                    login: "bob".into(),
                    ..UserRule::default()
                },
            ],
            badges: vec![
                BadgeRule::default(),
                BadgeRule {
                    badge: "vip".into(),
                    ..BadgeRule::default()
                },
            ],
            ignores: vec![
                IgnoreRule::default(),
                IgnoreRule {
                    pattern: "x".into(),
                    ..IgnoreRule::default()
                },
            ],
            filters: vec![
                FilterDef {
                    id: "a".into(),
                    ..FilterDef::default()
                },
                FilterDef {
                    id: "b".into(),
                    name: "n".into(),
                    ..FilterDef::default()
                },
            ],
            ..Highlights::default()
        };
        assert!(h.prune_blank());
        assert_eq!(h.phrases.len(), 1);
        assert_eq!(h.users.len(), 1);
        assert_eq!(h.badges.len(), 1);
        assert_eq!(h.ignores.len(), 1);
        assert_eq!(h.filters.len(), 1);
        assert_eq!(h.filters[0].id, "b");
        assert!(!h.prune_blank());
    }

    // V105-17
    #[test]
    fn alert_keeps_a_sound_file_and_drops_others() {
        let mut h = Highlights {
            phrases: vec![
                PhraseRule {
                    pattern: "a".into(),
                    alert: Alert {
                        sound_file: "C:\\x.mp3".into(),
                        show: true,
                        ..Alert::default()
                    },
                    ..PhraseRule::default()
                },
                PhraseRule {
                    pattern: "b".into(),
                    alert: Alert {
                        sound_file: "C:\\x.exe".into(),
                        ..Alert::default()
                    },
                    ..PhraseRule::default()
                },
            ],
            ..Highlights::default()
        };
        h.sanitize();
        assert_eq!(h.phrases[0].alert.sound_file, "C:\\x.mp3");
        assert_eq!(h.phrases[1].alert.sound_file, "");
        let json = serde_json::to_value(&h.phrases[0].alert).unwrap();
        assert_eq!(json["show"], true);
        assert_eq!(json["soundFile"], "C:\\x.mp3");
    }

    // V106-09, V106-33
    #[test]
    fn defaults_your_name_sounds_and_both_go_to_mentions() {
        let h: Highlights = serde_json::from_str("{}").unwrap();
        assert!(h.me.enabled && h.me.alert.sound && h.me.alert.show);
        assert!(h.replies.enabled && !h.replies.alert.sound && h.replies.alert.show);
        assert!(h.mentions_merged && h.duplicates_asked);
        let p: PhraseRule = serde_json::from_str(r#"{"pattern":"a"}"#).unwrap();
        assert!(p.whole_word && !p.lookalikes && !p.alert.show && p.flavor == RegexFlavor::Rust);
    }

    // V106-33 Q4
    #[test]
    fn your_name_and_replies_stay_in_the_mentions_tab() {
        let mut h: Highlights = serde_json::from_value(serde_json::json!({
            "me": {"show": false}, "replies": {"show": false}, "mentionsMerged": false
        }))
        .unwrap();
        h.sanitize();
        assert!(h.me.alert.show && h.replies.alert.show && h.mentions_merged);
    }

    #[test]
    fn caps_words_and_global_mentions() {
        let rule = |p: String| PhraseRule {
            pattern: p,
            ..PhraseRule::default()
        };
        let mut h = Highlights {
            phrases: (0..310).map(|i| rule(format!("w{i}"))).collect(),
            global_list: (0..105).map(|i| rule(format!("g{i}"))).collect(),
            ..Highlights::default()
        };
        h.global_list[0].pattern = "x".repeat(600);
        h.global_list[1].alert.sound_file = r"C:\a\ping.exe".into();
        h.sanitize();
        assert_eq!((h.phrases.len(), h.global_list.len()), (MAX_WORDS, MAX_GLOBAL));
        assert_eq!(h.global_list[0].pattern.len(), 500);
        assert_eq!(h.global_list[1].alert.sound_file, "", "not a sound file");
    }

    #[test]
    fn global_needs_a_pattern() {
        let mut h = Highlights {
            global: true,
            ..Highlights::default()
        };
        assert!(!h.global_active());
        h.global_list.push(PhraseRule {
            pattern: "  ".into(),
            ..PhraseRule::default()
        });
        assert!(!h.global_active());
        assert!(h.prune_blank(), "blank global rows go too");
        h.global_list.push(PhraseRule {
            pattern: "cyex".into(),
            ..PhraseRule::default()
        });
        assert!(h.global_active());
        h.global = false;
        assert!(!h.global_active());
    }

    #[test]
    fn alerts_cover_every_rule() {
        let mut h = Highlights::default();
        h.global_list.push(PhraseRule {
            pattern: "g".into(),
            alert: Alert {
                route: "Global".into(),
                ..Alert::default()
            },
            ..PhraseRule::default()
        });
        h.replies.alert.route = "Replies".into();
        assert_eq!(h.alerts().count(), 9);
        assert_eq!(h.routes(), ["Global", "Replies"]);
    }

    #[test]
    fn dup_key_ignores_case_spaces_and_flavor_of_plain_words() {
        let r = |p: &str, regex: bool, flavor: RegexFlavor| PhraseRule {
            pattern: p.into(),
            regex,
            flavor,
            ..PhraseRule::default()
        };
        assert_eq!(r(" Cyex ", false, RegexFlavor::Rust).dup_key(), r("cyex", false, RegexFlavor::Python).dup_key());
        assert_ne!(r("cyex", false, RegexFlavor::Rust).dup_key(), r("cyex", true, RegexFlavor::Rust).dup_key());
        assert_ne!(r("a+", true, RegexFlavor::Rust).dup_key(), r("a+", true, RegexFlavor::JavaScript).dup_key());
    }
}
