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
mod tests;
