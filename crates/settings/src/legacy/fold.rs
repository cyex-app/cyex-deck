//! V106-33 Folds the old `mentions` partition and pre-v1.06 highlights into today's
//! Highlights: on the first start after the update, and on import or restore of old
//! exports, share codes and backups. Works on raw values: once normalized, every value
//! looks merged.

use std::collections::HashSet;

use serde_json::Value;

use super::mentions::{OldAlert, OldMentionRule, OldMentions, OldYourName};
use crate::schema::{Alert, BuiltIn, Highlights, PhraseRule, RegexFlavor, Schema, MAX_GLOBAL, MAX_WORDS};

/// Where the old values come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The settings folder or one of its backups: holds every partition it ever wrote.
    Disk,
    /// An export or share code: may hold only some partitions.
    Bundle,
}

/// V106-33 What the merge did, for the log.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MergeReport {
    pub from_mentions: usize,
    pub from_highlights: usize,
    pub global: usize,
    pub duplicates: usize,
    pub cut: usize,
}

/// Raw values (before normalizing). True when they still need merging.
pub fn needs_fold(highlights: Option<&Value>, mentions_present: bool) -> bool {
    match highlights {
        None => mentions_present,
        Some(h) => h.get("mentionsMerged") != Some(&Value::Bool(true)),
    }
}

/// `current`: the store's Highlights, used for what a bundle doesn't carry.
pub fn fold(
    highlights: Option<Value>,
    mentions: Option<Value>,
    current: &Highlights,
    source: Source,
) -> serde_json::Result<(Highlights, MergeReport)> {
    let m: Option<OldMentions> = mentions.map(serde_json::from_value).transpose()?;
    let mut report = MergeReport::default();
    // Whether `me` comes from old data (V106-09 turns its sound on) or stays the current one.
    let mut old_me = m.is_some();
    let mut h = match highlights {
        Some(raw) => {
            let old_mention: BuiltIn = raw
                .get("mention")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            let mut h: Highlights = serde_json::from_value(raw)?;
            // Old phrases: plain words matched whole words, regexes ran as written; one
            // look-alike switch for all (Q11).
            let old_lookalikes = h.lookalikes;
            for p in &mut h.phrases {
                p.whole_word = !p.regex;
                p.lookalikes = old_lookalikes;
                p.flavor = RegexFlavor::Rust;
                p.alert.notify = false;
            }
            report.from_highlights = h.phrases.len();
            if let Some(m) = &m {
                h.phrases.splice(0..0, m.list.iter().map(word));
                report.from_mentions = m.list.len();
            } else if source == Source::Disk {
                // Before v1.04 your name was a highlight: switch, color, sound and flash carry over.
                h.me = BuiltIn {
                    enabled: old_mention.enabled,
                    alert: Alert {
                        color: old_mention.alert.color,
                        sound: old_mention.alert.sound,
                        flash: old_mention.alert.flash,
                        show: true,
                        ..Alert::default()
                    },
                };
                h.replies = h.me.clone();
                h.global = false;
                h.global_list.clear();
                old_me = true;
            } else {
                // A highlights-only export never carried these; v1.05 left Mentions alone.
                h.me = current.me.clone();
                h.replies = current.replies.clone();
                h.global = current.global;
                h.global_list = current.global_list.clone();
            }
            h
        }
        None => {
            let mut h = current.clone();
            if let Some(m) = &m {
                let mut words = clean(m.list.iter().map(word).collect());
                words.retain(|w| !h.phrases.contains(w));
                report.from_mentions = words.len();
                h.phrases.splice(0..0, words);
            }
            h
        }
    };
    if let Some(m) = &m {
        h.me = your_name(&m.me);
        h.replies = h.me.clone();
        h.global = m.global;
        h.global_list = m.global_list.iter().map(word).collect();
        report.global = h.global_list.len();
    }
    // V106-09 Q9 Your name plays a sound, for every user, once.
    if old_me {
        h.me.alert.sound = true;
    }
    report.duplicates = repeats(&h.phrases) + repeats(&h.global_list);
    if report.duplicates > 0 {
        h.duplicates_asked = false;
    }
    report.cut = h.phrases.len().saturating_sub(MAX_WORDS) + h.global_list.len().saturating_sub(MAX_GLOBAL);
    h.sanitize();
    Ok((h, report))
}

/// Rows with an earlier row of the same `dup_key`. Nothing is removed (Q13).
fn repeats(list: &[PhraseRule]) -> usize {
    let mut seen = HashSet::new();
    list.iter()
        .filter(|r| !r.is_blank())
        .filter(|r| !seen.insert(r.dup_key()))
        .count()
}

/// Runs rows through sanitize, so they compare equal to stored ones.
fn clean(words: Vec<PhraseRule>) -> Vec<PhraseRule> {
    let mut h = Highlights {
        phrases: words,
        ..Highlights::default()
    };
    h.sanitize();
    h.phrases
}

fn alert(a: &OldAlert) -> Alert {
    Alert {
        color: a.color.clone(),
        sound: a.sound,
        flash: a.flash,
        show: a.show,
        notify: a.notify,
        sound_file: a.sound_file.clone(),
        route: String::new(),
    }
}

fn your_name(me: &OldYourName) -> BuiltIn {
    BuiltIn {
        enabled: me.enabled,
        alert: alert(&me.alert),
    }
}

fn word(r: &OldMentionRule) -> PhraseRule {
    PhraseRule {
        enabled: r.enabled,
        pattern: r.pattern.clone(),
        regex: r.regex,
        flavor: r.flavor,
        case_sensitive: r.case_sensitive,
        whole_word: r.whole_word,
        lookalikes: r.lookalikes,
        alert: alert(&r.alert),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn empty() -> Value {
        json!({})
    }

    fn disk(h: Value, m: Option<Value>) -> (Highlights, MergeReport) {
        fold(Some(h), m, &Highlights::default(), Source::Disk).unwrap()
    }

    #[test]
    fn needs_fold_reads_the_raw_flag() {
        assert!(!needs_fold(None, false));
        assert!(needs_fold(None, true));
        assert!(needs_fold(Some(&empty()), false));
        assert!(needs_fold(Some(&json!({"mentionsMerged": false})), false));
        assert!(!needs_fold(Some(&json!({"mentionsMerged": true})), true));
    }

    #[test]
    fn pre_104_highlights_only() {
        let (h, r) = disk(
            json!({
                "mention": {"enabled": true, "color": "#ff000080", "sound": false, "flash": true,
                            "soundFile": "C:\\a.wav", "route": "Pings"},
                "lookalikes": true,
                "phrases": [{"pattern": "hello"}, {"pattern": "a+b", "regex": true, "show": true}]
            }),
            None,
        );
        assert!(h.me.enabled && h.me.alert.flash && h.me.alert.show);
        assert_eq!(h.me.alert.color, "#ff000080");
        assert_eq!(
            (h.me.alert.sound_file.as_str(), h.me.alert.route.as_str()),
            ("", "")
        );
        assert!(h.me.alert.sound, "V106-09");
        assert!(!h.replies.alert.sound && h.replies.alert.show && h.replies.alert.flash);
        assert_eq!(h.replies.alert.color, "#ff000080");
        let [plain, re] = &h.phrases[..] else { panic!() };
        assert!(plain.whole_word && plain.lookalikes && !plain.alert.show);
        assert!(!re.whole_word && re.lookalikes && re.alert.show && re.flavor == RegexFlavor::Rust);
        assert_eq!(
            r,
            MergeReport {
                from_highlights: 2,
                ..MergeReport::default()
            }
        );
        assert!(h.mentions_merged && h.duplicates_asked);
    }

    #[test]
    fn large() {
        let mentions: Vec<Value> = (0..100)
            .map(|i| {
                json!({"pattern": format!("m{i}"), "wholeWord": i % 2 == 0, "caseSensitive": i % 3 == 0,
                       "lookalikes": i % 5 == 0, "enabled": i % 7 != 0, "show": i % 4 != 0,
                       "notify": i % 6 == 0, "flash": true, "color": "#112233"})
            })
            .collect();
        let phrases: Vec<Value> = (0..200)
            .map(|i| json!({"pattern": format!("p{i}"), "sound": true}))
            .collect();
        let (h, r) = disk(json!({"phrases": phrases}), Some(json!({"list": mentions})));
        assert_eq!(h.phrases.len(), 300);
        assert_eq!(
            (r.from_mentions, r.from_highlights, r.cut, r.duplicates),
            (100, 200, 0, 0)
        );
        assert!(h.phrases[..100]
            .iter()
            .enumerate()
            .all(|(i, p)| p.pattern == format!("m{i}")));
        let m9 = &h.phrases[9];
        assert!(!m9.whole_word && m9.case_sensitive && !m9.lookalikes && m9.enabled && m9.alert.show);
        assert!(h.phrases[0].alert.notify && !h.phrases[0].enabled && !h.phrases[0].alert.show);
        assert_eq!(h.phrases[0].alert.color, "#112233");
        assert!(h.phrases[100].alert.sound && h.phrases[100].whole_word);
        let back: Highlights = serde_json::from_value(serde_json::to_value(&h).unwrap()).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn regex_flavors() {
        let list = json!([
            {"pattern": "a+", "regex": true, "flavor": "rust"},
            {"pattern": "b+", "regex": true, "flavor": "javascript"},
            {"pattern": "c+", "regex": true, "flavor": "python"}
        ]);
        let (h, _) = disk(
            json!({"phrases": [{"pattern": "d+", "regex": true}]}),
            Some(json!({"list": list})),
        );
        let flavors: Vec<_> = h.phrases.iter().map(|p| (p.regex, p.flavor)).collect();
        assert_eq!(
            flavors,
            [
                (true, RegexFlavor::Rust),
                (true, RegexFlavor::JavaScript),
                (true, RegexFlavor::Python),
                (true, RegexFlavor::Rust)
            ]
        );
        assert!(!h.phrases[3].whole_word);
    }

    #[test]
    fn custom_sounds() {
        let (h, _) = disk(
            empty(),
            Some(json!({
                "me": {"soundFile": "C:\\s\\me.wav"},
                "list": [{"pattern": "w", "soundFile": "C:\\s\\w.mp3"}, {"pattern": "x", "soundFile": "C:\\x.exe"}],
                "global": true,
                "globalList": [{"pattern": "g", "soundFile": "C:\\s\\g.ogg"}]
            })),
        );
        assert_eq!(h.me.alert.sound_file, "C:\\s\\me.wav");
        assert_eq!(h.replies.alert.sound_file, "C:\\s\\me.wav");
        assert_eq!(h.phrases[0].alert.sound_file, "C:\\s\\w.mp3");
        assert_eq!(h.phrases[1].alert.sound_file, "", "not a sound file");
        assert!(h.global);
        assert_eq!(h.global_list[0].alert.sound_file, "C:\\s\\g.ogg");
    }

    #[test]
    fn duplicates_counted_not_removed() {
        let (h, r) = disk(
            json!({"phrases": [{"pattern": "cyex "}, {"pattern": ""}, {"pattern": " "}]}),
            Some(json!({"list": [{"pattern": "Cyex"}], "globalList": [{"pattern": "a"}, {"pattern": "A"}]})),
        );
        assert_eq!(h.phrases.len(), 4);
        assert_eq!(h.global_list.len(), 2);
        assert_eq!(r.duplicates, 2, "blank rows aren't duplicates");
        assert!(!h.duplicates_asked);
    }

    // V106-09 Q9
    #[test]
    fn name_sound_turned_on() {
        let (h, _) = disk(empty(), Some(json!({"me": {"sound": false, "color": "#00ff00"}})));
        assert!(h.me.alert.sound);
        assert_eq!(h.me.alert.color, "#00ff00");
        assert!(!h.replies.alert.sound);
        assert_eq!(h.replies.alert.color, "#00ff00");
    }

    // Q4
    #[test]
    fn name_and_replies_always_in_mentions_tab() {
        let (h, _) = disk(empty(), Some(json!({"me": {"show": false}})));
        assert!(h.me.alert.show && h.replies.alert.show);
    }

    #[test]
    fn highlights_only_bundle_keeps_current_name_and_global() {
        let mut current = Highlights {
            global: true,
            global_list: vec![PhraseRule {
                pattern: "g".into(),
                ..PhraseRule::default()
            }],
            ..Highlights::default()
        };
        current.me.alert.sound = false;
        current.me.alert.color = "#123456".into();
        let (h, r) = fold(
            Some(json!({"mention": {"enabled": false}, "phrases": [{"pattern": "hi"}]})),
            None,
            &current,
            Source::Bundle,
        )
        .unwrap();
        assert_eq!(h.me, current.me, "no sound forced on a v1.06 choice");
        assert_eq!(h.replies, current.replies);
        assert!(h.global && h.global_list == current.global_list);
        assert!(h.phrases[0].whole_word);
        assert_eq!(r.from_highlights, 1);
    }

    #[test]
    fn mentions_only_bundle_prepends_and_skips_exact_copies() {
        let (current, _) = disk(
            empty(),
            Some(json!({"list": [{"pattern": "a"}, {"pattern": "b"}]})),
        );
        let mut current = current;
        current.phrases.push(PhraseRule {
            pattern: "p".into(),
            ..PhraseRule::default()
        });
        current.users.push(crate::schema::UserRule {
            login: "bob".into(),
            ..Default::default()
        });
        let (h, r) = fold(
            None,
            Some(json!({"me": {"color": "#abcdef"}, "list": [{"pattern": "a"}, {"pattern": "c"}], "global": true})),
            &current,
            Source::Bundle,
        )
        .unwrap();
        let patterns: Vec<_> = h.phrases.iter().map(|p| p.pattern.as_str()).collect();
        assert_eq!(patterns, ["c", "a", "b", "p"]);
        assert_eq!(r.from_mentions, 1);
        assert_eq!(h.me.alert.color, "#abcdef");
        assert!(h.global && h.users == current.users);
    }

    #[test]
    fn a_broken_mentions_value_is_an_error() {
        assert!(fold(
            None,
            Some(json!({"list": 3})),
            &Highlights::default(),
            Source::Bundle
        )
        .is_err());
    }
}
