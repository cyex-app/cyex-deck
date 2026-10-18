use super::*;
use crate::build::Me;
use crate::irc::Line;
use crate::model::{HighlightKind, RoomEvent};
use crate::similar::RepeatCheck;
use crate::tokenize::NoEmotes;
use cyex_settings::schema::{Alert, Highlights, IgnoreRule, PhraseRule, Similarity};

fn feed(r: &mut Room, id: &str, user: &str, text: &str) -> Vec<RoomEvent> {
    let raw = format!("@id={id};user-id={user};tmi-sent-ts={id}000 :u{user}!u@u PRIVMSG #chan :{text}");
    r.handle_line(&Line::parse(&raw).unwrap(), 1_000, &NoEmotes)
}

#[test]
fn blank_rules_match_nothing() {
    use cyex_settings::schema::{BadgeRule, UserRule};
    let h = Highlights {
        phrases: vec![PhraseRule::default()],
        users: vec![UserRule::default()],
        badges: vec![BadgeRule::default()],
        ignores: vec![IgnoreRule {
            block: true,
            ..IgnoreRule::default()
        }],
        ..Highlights::default()
    };
    let mut r = Room::new("chan", Some(Me::new("42", "me", "Me")));
    r.set_rules(Arc::new(Rules::new(&h)));
    let ev = feed(&mut r, "1", "7", "hello");
    let RoomEvent::Message { message } = &ev[0] else {
        panic!("blocked by a blank ignore")
    };
    assert!(message.meta.highlight.is_none());
}

#[test]
fn rules_hide_replace_highlight_and_flag() {
    let h = Highlights {
        phrases: vec![
            // V104-01 An old mention: a pop-up, the Mentions tab.
            PhraseRule {
                pattern: "pasta".into(),
                alert: Alert {
                    notify: true,
                    show: true,
                    ..Alert::default()
                },
                ..PhraseRule::default()
            },
            PhraseRule {
                pattern: "pizza".into(),
                alert: Alert {
                    sound: true,
                    route: "Food".into(),
                    ..Default::default()
                },
                ..PhraseRule::default()
            },
        ],
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
    let mut rules = Rules::new(&h);
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

    // V106-33 Q10 A word with the Mentions tab on: flagged like your name.
    let ev = feed(&mut r, "6", "9", "PASTA later?");
    let RoomEvent::Message { message } = &ev[0] else {
        panic!()
    };
    assert!(message.has(Flag::Mention));
    let out = r.take_outcomes();
    let (_, o) = out.last().unwrap();
    assert!(o.notify && !o.sound && o.routes == ["Mentions"]);
}

// V105-17 A highlight can copy into Mentions and bring its own sound file.
#[test]
fn highlights_show_in_mentions_and_play_their_file() {
    let phrase = |p: &str, show: bool, file: &str| PhraseRule {
        pattern: p.into(),
        alert: Alert {
            sound: true,
            show,
            sound_file: file.into(),
            ..Alert::default()
        },
        ..PhraseRule::default()
    };
    let h = Highlights {
        phrases: vec![
            phrase("alpha", true, "C:\\a.wav"),
            phrase("beta", true, "C:\\b.wav"),
            phrase("gamma", false, ""),
        ],
        ..Highlights::default()
    };
    let mut r = Room::new("chan", Some(Me::new("42", "me", "Me")));
    r.set_rules(Arc::new(Rules::new(&h)));

    feed(&mut r, "1", "7", "alpha");
    let out = r.take_outcomes();
    assert_eq!(out[0].1.routes, ["Mentions"]);
    assert_eq!(out[0].1.sound_file.as_deref(), Some("C:\\a.wav"));

    feed(&mut r, "2", "7", "beta alpha");
    let out = r.take_outcomes();
    assert_eq!(out[0].1.routes, ["Mentions"], "once");
    assert_eq!(out[0].1.sound_file.as_deref(), Some("C:\\a.wav"), "the first one's file");

    feed(&mut r, "3", "7", "gamma");
    let out = r.take_outcomes();
    assert!(out[0].1.routes.is_empty() && out[0].1.sound_file.is_none());
}
// V106-33 Rooms as the engine sees them: me is `cyex` (id 42).
fn cyex_room(h: &Highlights) -> Room {
    let mut r = Room::new("chan", Some(Me::new("42", "cyex", "Cyex")));
    r.set_rules(Arc::new(Rules::new(h)));
    r
}

/// A reply to `parent` from user 7.
fn reply(r: &mut Room, id: &str, parent: &str, text: &str) -> Vec<RoomEvent> {
    let raw = format!(
        "@id={id};user-id=7;tmi-sent-ts={id}000;reply-parent-msg-id=p;reply-parent-user-login={parent};reply-parent-display-name={parent};reply-parent-msg-body=x;reply-thread-parent-msg-id=p :u7!u@u PRIVMSG #chan :@{parent} {text}"
    );
    r.handle_line(&Line::parse(&raw).unwrap(), 1_000, &NoEmotes)
}

fn message(ev: &[RoomEvent]) -> &ChatMessage {
    match ev.iter().find(|e| matches!(e, RoomEvent::Message { .. })) {
        Some(RoomEvent::Message { message }) => message,
        _ => panic!("{ev:?}"),
    }
}

fn alert(color: &str, sound: bool, file: &str, route: &str) -> Alert {
    Alert {
        color: color.into(),
        sound,
        sound_file: file.into(),
        route: route.into(),
        ..Alert::default()
    }
}

fn word(p: &str, a: Alert) -> PhraseRule {
    PhraseRule {
        pattern: p.into(),
        alert: a,
        ..PhraseRule::default()
    }
}

// V106-33 First match sets the color; sound, flash, pop-up OR-ed; first file wins; routes once.
#[test]
fn precedence() {
    use cyex_settings::schema::{BadgeRule, BuiltIn, UserRule};
    let h = Highlights {
        me: BuiltIn {
            enabled: true,
            alert: alert("#111111", false, "", "Feed"),
        },
        replies: BuiltIn {
            enabled: true,
            alert: Alert {
                flash: true,
                ..alert("#222222", true, "C:\\reply.wav", "")
            },
        },
        phrases: vec![
            word("one", Alert {
                notify: true,
                ..alert("#333333", true, "C:\\one.wav", "Feed")
            }),
            word("two", alert("#444444", false, "", "Other")),
        ],
        users: vec![UserRule {
            login: "u7".into(),
            alert: alert("#555555", false, "", "Feed"),
            ..UserRule::default()
        }],
        badges: vec![BadgeRule {
            badge: "vip".into(),
            alert: alert("#666666", false, "", ""),
            ..BadgeRule::default()
        }],
        ..Highlights::default()
    };
    let mut r = cyex_room(&h);
    let raw = "@id=1;user-id=7;badges=vip/1;tmi-sent-ts=1000;reply-parent-msg-id=p;reply-parent-user-login=cyex;reply-parent-display-name=Cyex;reply-parent-msg-body=x;reply-thread-parent-msg-id=p :u7!u@u PRIVMSG #chan :@Cyex cyex one two";
    let ev = r.handle_line(&Line::parse(raw).unwrap(), 1_000, &NoEmotes);
    let m = message(&ev);
    let hl = m.meta.highlight.as_ref().unwrap();
    assert_eq!((hl.kind, hl.color.as_deref()), (HighlightKind::Mention, Some("#111111")));
    assert!(m.has(Flag::Mention));
    let out = r.take_outcomes();
    let o = &out[0].1;
    assert!(o.sound && o.flash && o.notify);
    assert_eq!(o.sound_file.as_deref(), Some("C:\\reply.wav"));
    assert_eq!(o.routes, ["Feed", "Mentions", "Other"]);
}

// V106-33
#[test]
fn name_and_reply_are_separate() {
    use cyex_settings::schema::BuiltIn;
    let h = Highlights {
        me: BuiltIn {
            enabled: true,
            alert: alert("#111111", false, "", ""),
        },
        replies: BuiltIn {
            enabled: true,
            alert: alert("#222222", false, "", ""),
        },
        ..Highlights::default()
    };
    let mut r = cyex_room(&h);
    let color = |ev: &[RoomEvent]| message(ev).meta.highlight.as_ref().and_then(|h| h.color.clone());
    assert_eq!(color(&reply(&mut r, "1", "cyex", "thanks")).as_deref(), Some("#222222"), "reply only");
    assert_eq!(color(&feed(&mut r, "2", "7", "hi cyex")).as_deref(), Some("#111111"), "name only");
    let both = color(&reply(&mut r, "3", "cyex", "yes cyex"));
    assert_eq!(both.as_deref(), Some("#111111"), "both: your name");
    let out = r.take_outcomes();
    assert!(out.iter().all(|(_, o)| o.routes == ["Mentions"]), "always in the Mentions tab");
}

// V106-33 O2 A row that's off matches nothing: no color, no sound, no Mentions tab, no flag.
#[test]
fn off_rows_are_fully_off() {
    let mut h = Highlights::default();
    h.me.enabled = false;
    h.replies.enabled = false;
    let mut r = cyex_room(&h);
    for ev in [feed(&mut r, "1", "7", "hi cyex"), reply(&mut r, "2", "cyex", "ok")] {
        let m = message(&ev);
        assert!(m.meta.highlight.is_none() && !m.has(Flag::Mention), "{m:?}");
        assert!(m.has(Flag::NamesMe) || m.has(Flag::RepliesToMe));
    }
    let out = r.take_outcomes();
    assert!(out.iter().all(|(_, o)| *o == Outcome::default()));
    // Only replies off: your name still counts.
    h.me.enabled = true;
    let mut r = cyex_room(&h);
    assert!(message(&feed(&mut r, "3", "7", "cyex!")).has(Flag::Mention));
    assert!(!message(&reply(&mut r, "4", "cyex", "ok")).has(Flag::Mention));
    // Default: your name plays a sound (V106-09), replies don't.
    let mut r = cyex_room(&Highlights::default());
    feed(&mut r, "5", "7", "cyex");
    reply(&mut r, "6", "cyex", "ok");
    let out = r.take_outcomes();
    assert!(out[0].1.sound && !out[1].1.sound);
}

// V106-33 Q10
#[test]
fn word_with_mentions_tab_is_a_mention() {
    let h = Highlights {
        phrases: vec![
            word("alpha", Alert {
                show: true,
                ..Alert::default()
            }),
            word("beta", Alert::default()),
        ],
        ..Highlights::default()
    };
    let mut r = cyex_room(&h);
    let ev = feed(&mut r, "1", "7", "alpha");
    let m = message(&ev);
    assert!(m.has(Flag::Mention) && m.meta.highlight.as_ref().unwrap().kind == HighlightKind::Mention);
    let ev = feed(&mut r, "2", "7", "beta");
    let m = message(&ev);
    assert!(!m.has(Flag::Mention) && m.meta.highlight.as_ref().unwrap().kind == HighlightKind::Rule);
    let ev = r.handle_line(
        &Line::parse("@id=3;user-id=42;tmi-sent-ts=3000 :cyex!cyex@cyex PRIVMSG #chan :alpha").unwrap(),
        1_000,
        &NoEmotes,
    );
    assert!(!message(&ev).has(Flag::Mention), "own messages never");
}

// V106-33 Word rules keep their own look-alikes, also on highlights.
#[test]
fn words_fold_per_row() {
    let h = Highlights {
        phrases: vec![PhraseRule {
            lookalikes: true,
            ..word("paypal", Alert::default())
        }],
        ..Highlights::default()
    };
    let mut r = cyex_room(&h);
    assert!(message(&feed(&mut r, "1", "7", "free раураl")).meta.highlight.is_some());
}
