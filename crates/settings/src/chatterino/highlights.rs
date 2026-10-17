//! V105-16 Highlights from Chatterino's settings.json: phrases, users and badges only. The file holds account tokens: only the highlight lists are read, nothing else is kept, and nothing from the file is ever logged.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::schema::{
    Alert, BadgeRule, Highlights, MAX_RULES, MAX_WORDS, PhraseRule, RegexFlavor, UserRule, is_sound_file,
};

const FALLBACK_COLOR: &str = "#7f3f497f";

/// The file isn't Chatterino's settings.json.
#[derive(Debug, PartialEq, Eq)]
pub struct NotSettings;

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    highlighting: Option<Section>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(default)]
    highlights: Vec<Phrase>,
    #[serde(default)]
    users: Vec<Phrase>,
    #[serde(default)]
    badges: Vec<Badge>,
}

#[derive(Deserialize)]
struct Phrase {
    #[serde(default)]
    pattern: String,
    #[serde(default = "yes", rename = "showInMentions")]
    show_in_mentions: bool,
    #[serde(default = "yes")]
    alert: bool,
    #[serde(default)]
    sound: bool,
    #[serde(default)]
    regex: bool,
    #[serde(default, rename = "case")]
    case: bool,
    #[serde(default, rename = "soundUrl")]
    sound_url: String,
    #[serde(default)]
    color: String,
}

#[derive(Deserialize)]
struct Badge {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "showInMentions")]
    show_in_mentions: bool,
    #[serde(default = "yes")]
    alert: bool,
    #[serde(default)]
    sound: bool,
    #[serde(default, rename = "soundUrl")]
    sound_url: String,
    #[serde(default)]
    color: String,
}

/// A rule that can't be imported, with Chatterino's pattern and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Skipped {
    pub pattern: String,
    pub reason: String,
}

fn skipped(pattern: &str, reason: &str) -> Skipped {
    Skipped {
        pattern: pattern.to_owned(),
        reason: reason.to_owned(),
    }
}

/// What the file holds, as our rules.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    pub phrases: Vec<PhraseRule>,
    pub users: Vec<UserRule>,
    pub badges: Vec<BadgeRule>,
    pub invalid: Vec<Skipped>,
}

/// `#AARRGGBB` to `#rrggbbaa`, `#RRGGBB` as is, anything else Chatterino's fallback.
pub(crate) fn color(argb: &str) -> String {
    let hex = argb.strip_prefix('#').unwrap_or("");
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return FALLBACK_COLOR.to_owned();
    }
    let hex = hex.to_ascii_lowercase();
    match hex.len() {
        8 => format!("#{}{}", &hex[2..], &hex[..2]),
        6 => format!("#{hex}"),
        _ => FALLBACK_COLOR.to_owned(),
    }
}

/// A `file:///` URL to a Windows path, or empty when it isn't a sound file.
pub(crate) fn sound_file(url: &str) -> String {
    let Some(path) = url::Url::parse(url)
        .ok()
        .filter(|u| u.scheme() == "file")
        .and_then(|u| u.to_file_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
    else {
        return String::new();
    };
    if is_sound_file(&path) { path } else { String::new() }
}

fn alert(alert: bool, sound: bool, show: bool, sound_url: &str, color_text: &str) -> Alert {
    Alert {
        color: color(color_text),
        sound,
        flash: alert,
        show,
        notify: false,
        sound_file: sound_file(sound_url),
        route: String::new(),
    }
}

/// `regex` turns a PCRE2 pattern into the one we store (the app passes the chat crate's).
pub fn parse(text: &str, regex: impl Fn(&str) -> Result<String, String>) -> Result<Found, NotSettings> {
    let file: File = serde_json::from_str(text).map_err(|_| NotSettings)?;
    let section = file.highlighting.ok_or(NotSettings)?;
    let mut found = Found::default();
    for p in section.highlights {
        let pattern = p.pattern.trim();
        let Some(stored) = stored(pattern, p.regex, &regex, &mut found.invalid) else {
            continue;
        };
        found.phrases.push(PhraseRule {
            enabled: true,
            pattern: stored,
            regex: p.regex,
            flavor: RegexFlavor::Rust,
            case_sensitive: p.case,
            whole_word: !p.regex,
            lookalikes: false,
            alert: alert(p.alert, p.sound, p.show_in_mentions, &p.sound_url, &p.color),
        });
    }
    for u in section.users {
        let login = u.pattern.trim();
        let Some(mut stored) = stored(login, u.regex, &regex, &mut found.invalid) else {
            continue;
        };
        if !u.regex {
            stored.make_ascii_lowercase();
        }
        found.users.push(UserRule {
            enabled: true,
            login: stored,
            regex: u.regex,
            alert: alert(u.alert, u.sound, u.show_in_mentions, &u.sound_url, &u.color),
        });
    }
    for b in section.badges {
        let parts: Vec<String> = b
            .name
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            found.invalid.push(skipped(&b.name, "Empty pattern."));
        }
        for badge in parts {
            found.badges.push(BadgeRule {
                enabled: true,
                badge,
                alert: alert(b.alert, b.sound, b.show_in_mentions, &b.sound_url, &b.color),
            });
        }
    }
    Ok(found)
}

/// The pattern as stored, or `None` after noting why it can't be.
fn stored(
    pattern: &str,
    is_regex: bool,
    regex: &impl Fn(&str) -> Result<String, String>,
    invalid: &mut Vec<Skipped>,
) -> Option<String> {
    if pattern.is_empty() {
        invalid.push(skipped(pattern, "Empty pattern."));
        return None;
    }
    if !is_regex {
        return Some(pattern.to_owned());
    }
    match regex(pattern) {
        Ok(p) => Some(p),
        Err(reason) => {
            invalid.push(skipped(pattern, &reason));
            None
        }
    }
}

/// What importing would add, before the user decides about duplicates.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HighlightPreview {
    pub phrases: u32,
    pub users: u32,
    pub badges: u32,
    /// Patterns that exist already.
    pub duplicates: Vec<String>,
    pub invalid: Vec<Skipped>,
}

/// What an import did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HighlightImport {
    pub added: u32,
    pub replaced: u32,
    pub kept: u32,
    pub skipped: Vec<Skipped>,
}

#[derive(Default)]
struct Tally {
    added: u32,
    replaced: u32,
    kept: u32,
    skipped: Vec<Skipped>,
    duplicates: Vec<String>,
}

/// Same text ignoring case and surrounding spaces; for phrases also the same regex flag and flavor.
type Key = (String, bool, Option<RegexFlavor>);

/// Puts `new` into `list`: duplicates (same `key`) are kept or replaced in place, the rest
/// appended up to `max`.
fn merge<T>(
    list: &mut Vec<T>,
    new: Vec<T>,
    key: impl Fn(&T) -> Key,
    max: usize,
    keep_mine: bool,
    t: &mut Tally,
) {
    for rule in new {
        let k = key(&rule);
        if let Some(at) = list.iter().position(|r| key(r) == k) {
            t.duplicates.push(k.0);
            if keep_mine {
                t.kept += 1;
            } else {
                list[at] = rule;
                t.replaced += 1;
            }
        } else if list.len() >= max {
            t.skipped.push(skipped(&k.0, &format!("Over the {max} limit.")));
        } else {
            list.push(rule);
            t.added += 1;
        }
    }
}

fn norm(s: &str) -> Key {
    (s.trim().to_lowercase(), false, None)
}

fn run(found: Found, current: &mut Highlights, keep_mine: bool) -> (Tally, [u32; 3]) {
    let mut t = Tally::default();
    let before = [current.phrases.len(), current.users.len(), current.badges.len()];
    merge(
        &mut current.phrases,
        found.phrases,
        PhraseRule::dup_key,
        MAX_WORDS,
        keep_mine,
        &mut t,
    );
    merge(
        &mut current.users,
        found.users,
        |r| norm(&r.login),
        MAX_RULES,
        keep_mine,
        &mut t,
    );
    merge(
        &mut current.badges,
        found.badges,
        |r| norm(&r.badge),
        MAX_RULES,
        keep_mine,
        &mut t,
    );
    let grew = |now: usize, was: usize| u32::try_from(now - was).unwrap_or(0);
    let counts = [
        grew(current.phrases.len(), before[0]),
        grew(current.users.len(), before[1]),
        grew(current.badges.len(), before[2]),
    ];
    t.skipped.splice(0..0, found.invalid);
    (t, counts)
}

/// What importing `found` into `current` would do.
pub fn plan(found: &Found, current: &Highlights) -> HighlightPreview {
    let mut scratch = current.clone();
    let (t, [phrases, users, badges]) = run(found.clone(), &mut scratch, true);
    HighlightPreview {
        phrases,
        users,
        badges,
        duplicates: t.duplicates,
        invalid: t.skipped,
    }
}

/// Adds `found` to `current`; duplicates keep yours or take Chatterino's.
pub fn apply(found: Found, current: &mut Highlights, keep_mine: bool) -> HighlightImport {
    let (t, _) = run(found, current, keep_mine);
    HighlightImport {
        added: t.added,
        replaced: t.replaced,
        kept: t.kept,
        skipped: t.skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(p: &str) -> Result<String, String> {
        if p.contains("bad") {
            Err("Nope.".to_owned())
        } else {
            Ok(format!("R:{p}"))
        }
    }

    fn parse_ok(json: &str) -> Found {
        parse(json, fake).expect("parses")
    }

    const SAMPLE: &str = r##"{
        "accounts": {"uid1": {"oauthToken": "not-a-token"}},
        "highlighting": {
            "highlights": [
                {"pattern": "twitch.tv/", "showInMentions": false, "alert": false, "sound": false,
                 "regex": false, "case": false, "soundUrl": "", "color": "#ff0f398c"}
            ],
            "users": [{"pattern": "SomeUser"}],
            "badges": [{"name": "moderator,subscriber/3000", "displayName": "x"}]
        }
    }"##;

    #[test]
    fn maps_the_sample() {
        let f = parse_ok(SAMPLE);
        let p = &f.phrases[0];
        assert_eq!(p.pattern, "twitch.tv/");
        assert!(!p.regex && !p.case_sensitive && p.enabled);
        assert!(p.whole_word && !p.lookalikes && !p.alert.notify, "V106-33");
        assert!(!p.alert.show && !p.alert.flash && !p.alert.sound);
        assert_eq!(p.alert.sound_file, "");
        assert_eq!(p.alert.color, "#0f398cff");
        assert_eq!(f.users[0].login, "someuser");
        assert_eq!(f.badges.len(), 2);
        assert_eq!(f.badges[1].badge, "subscriber/3000");
        assert!(f.invalid.is_empty());
    }

    #[test]
    fn missing_fields_take_chatterinos_defaults() {
        let f =
            parse_ok(r#"{"highlighting": {"highlights": [{"pattern": "a"}], "badges": [{"name": "vip"}]}}"#);
        assert!(f.phrases[0].alert.show && f.phrases[0].alert.flash && !f.phrases[0].alert.sound);
        assert_eq!(f.phrases[0].alert.color, FALLBACK_COLOR);
        assert!(!f.badges[0].alert.show && f.badges[0].alert.flash);
    }

    #[test]
    fn regexes_go_through_the_converter() {
        let f = parse_ok(
            r#"{"highlighting": {"highlights": [
                {"pattern": "a+", "regex": true}, {"pattern": "bad(", "regex": true},
                {"pattern": "bad"}, {"pattern": "  "}],
              "users": [{"pattern": "^bot", "regex": true}]}}"#,
        );
        assert_eq!(f.phrases.len(), 2);
        assert_eq!(f.phrases[0].pattern, "R:a+");
        assert!(!f.phrases[0].whole_word, "regexes run as written");
        assert_eq!(f.phrases[1].pattern, "bad");
        assert_eq!(f.users[0].login, "R:^bot");
        assert_eq!(
            f.invalid,
            vec![skipped("bad(", "Nope."), skipped("", "Empty pattern.")]
        );
    }

    #[test]
    fn colors() {
        assert_eq!(color("#7f7f3f49"), "#7f3f497f");
        assert_eq!(color("#ff0000"), "#ff0000");
        assert_eq!(color("red"), "#7f3f497f");
        assert_eq!(color(""), "#7f3f497f");
    }

    #[test]
    fn sound_urls() {
        assert_eq!(sound_file("file:///C:/sounds/ping.wav"), r"C:\sounds\ping.wav");
        assert_eq!(sound_file("file:///C:/a%20b.mp3"), r"C:\a b.mp3");
        assert_eq!(sound_file("qrc:/sounds/ping2.wav"), "");
        assert_eq!(sound_file("file:///C:/x.exe"), "");
    }

    #[test]
    fn not_settings() {
        assert_eq!(parse("{}", fake).unwrap_err(), NotSettings);
        assert_eq!(parse("nope", fake).unwrap_err(), NotSettings);
    }

    fn rule(p: &str, regex: bool) -> PhraseRule {
        PhraseRule {
            pattern: p.into(),
            regex,
            ..PhraseRule::default()
        }
    }

    #[test]
    fn duplicates_are_kept_or_replaced_in_place() {
        let found = parse_ok(
            r#"{"highlighting": {"highlights": [{"pattern": "a+", "regex": true}, {"pattern": "new"}]}}"#,
        );
        let mut mine = Highlights {
            phrases: vec![rule("x", false), rule("R:a+", true)],
            ..Highlights::default()
        };
        let preview = plan(&found, &mine);
        assert_eq!((preview.phrases, preview.duplicates.len()), (1, 1));
        let kept = apply(found.clone(), &mut mine.clone(), true);
        assert_eq!((kept.added, kept.kept, kept.replaced), (1, 1, 0));
        let r = apply(found, &mut mine, false);
        assert_eq!((r.added, r.kept, r.replaced), (1, 0, 1));
        assert_eq!(mine.phrases[1].pattern, "R:a+");
        assert!(
            mine.phrases[1].alert.flash,
            "replaced by Chatterino's rule, in place"
        );
        assert_eq!(mine.phrases.len(), 3);
    }

    #[test]
    fn stops_at_the_limit() {
        let list: Vec<String> = (0..305).map(|i| format!(r#"{{"pattern": "w{i}"}}"#)).collect();
        let found = parse_ok(&format!(
            r#"{{"highlighting": {{"highlights": [{}]}}}}"#,
            list.join(",")
        ));
        let mut h = Highlights::default();
        let r = apply(found, &mut h, true);
        assert_eq!((h.phrases.len(), r.added, r.skipped.len()), (300, 300, 5));
        assert_eq!(r.skipped[0].reason, "Over the 300 limit.");
    }
}
