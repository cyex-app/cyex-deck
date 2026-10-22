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
    // V107-Mq Stored global rows always have it on.
    current.global_list[0].alert.show = true;
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
