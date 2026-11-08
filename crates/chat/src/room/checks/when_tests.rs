//! V110-C59 Rule conditions on real room lines.

use super::*;
use crate::build::Me;
use crate::irc::Line;
use crate::model::RoomEvent;
use crate::tokenize::NoEmotes;
use cyex_settings::schema::{
    Alert, BadgeRule, Highlights, IgnoreRule, LiveWhen, PhraseRule, RoleWhen, UserRule, When,
};

fn feed(r: &mut Room, id: &str, user: &str, text: &str) -> Vec<RoomEvent> {
    let raw = format!("@id={id};user-id={user};tmi-sent-ts={id}000 :u{user}!u@u PRIVMSG #chan :{text}");
    r.handle_line(&Line::parse(&raw).unwrap(), 1_000, &NoEmotes)
}

fn highlighted(ev: &[RoomEvent]) -> bool {
    matches!(ev.first(), Some(RoomEvent::Message { message }) if message.meta.highlight.is_some())
}

fn word(p: &str, when: When) -> PhraseRule {
    PhraseRule {
        pattern: p.into(),
        alert: Alert { color: "#ff0000".into(), ..Alert::default() },
        when: Some(when),
        ..PhraseRule::default()
    }
}

fn room(h: &Highlights) -> Room {
    let mut r = Room::new("chan", Some(Me::new("42", "me", "Me")));
    r.set_rules(Arc::new(Rules::new(h)));
    r
}

#[test]
fn channel_live_and_category() {
    let h = Highlights {
        phrases: vec![
            word("here", When { channels: vec!["chan".into()], ..When::default() }),
            word("elsewhere", When { channels: vec!["other".into()], ..When::default() }),
            word("live", When { live: Some(LiveWhen::Live), ..When::default() }),
            word("mc", When { categories: vec!["Minecraft".into()], ..When::default() }),
        ],
        ..Highlights::default()
    };
    let mut r = room(&h);
    assert!(highlighted(&feed(&mut r, "1", "7", "here")));
    assert!(!highlighted(&feed(&mut r, "2", "7", "elsewhere")));
    assert!(!highlighted(&feed(&mut r, "3", "7", "live")), "not known yet");
    r.set_stream(true, Some("Minecraft"));
    assert!(highlighted(&feed(&mut r, "4", "7", "live")));
    assert!(highlighted(&feed(&mut r, "5", "7", "mc")));
    r.set_stream(false, Some("Minecraft"));
    assert!(!highlighted(&feed(&mut r, "6", "7", "live")));
    assert!(!highlighted(&feed(&mut r, "7", "7", "mc")), "offline: no category");
}

#[test]
fn own_role_from_userstate() {
    let h = Highlights {
        users: vec![UserRule {
            login: "u7".into(),
            when: Some(When { role: Some(RoleWhen::Mod), ..When::default() }),
            ..UserRule::default()
        }],
        ..Highlights::default()
    };
    let mut r = room(&h);
    assert!(!highlighted(&feed(&mut r, "1", "7", "hi")));
    let us = Line::parse("@badges=moderator/1;mod=1 :tmi.twitch.tv USERSTATE #chan").unwrap();
    r.handle_line(&us, 1_000, &NoEmotes);
    assert!(highlighted(&feed(&mut r, "2", "7", "hi")));
}

#[test]
fn burst_ignore_hides_the_flood_only() {
    let h = Highlights {
        ignores: vec![IgnoreRule {
            pattern: "x".into(),
            block: true,
            when: Some(When { burst: 3, burst_secs: 5, ..When::default() }),
            ..IgnoreRule::default()
        }],
        ..Highlights::default()
    };
    let mut r = room(&h);
    assert!(!feed(&mut r, "1", "7", "x").is_empty());
    assert!(!feed(&mut r, "2", "7", "x").is_empty());
    assert!(feed(&mut r, "3", "7", "x").is_empty(), "third in 5 s: hidden");
    assert!(!feed(&mut r, "4", "8", "x").is_empty(), "someone else");
    assert!(!feed(&mut r, "20", "7", "x").is_empty(), "quiet for a while");
}

#[test]
fn first_message_only() {
    let h = Highlights {
        badges: vec![BadgeRule {
            badge: "vip".into(),
            when: Some(When { first_only: true, ..When::default() }),
            ..BadgeRule::default()
        }],
        ..Highlights::default()
    };
    let mut r = room(&h);
    let line = |id: &str, first: u8| {
        format!("@id={id};user-id=7;badges=vip/1;first-msg={first};tmi-sent-ts={id}000 :u7!u@u PRIVMSG #chan :hi")
    };
    let (a, b) = (line("1", 1), line("2", 0));
    assert!(highlighted(&r.handle_line(&Line::parse(&a).unwrap(), 1_000, &NoEmotes)));
    assert!(!highlighted(&r.handle_line(&Line::parse(&b).unwrap(), 1_000, &NoEmotes)));
}

#[test]
fn old_rules_without_when_still_apply() {
    let json = r##"{"phrases":[{"pattern":"hey","color":"#ff0000"}],"ignores":[{"pattern":"bad","block":true}]}"##;
    let h: Highlights = serde_json::from_str(json).unwrap();
    assert!(h.phrases[0].when.is_none() && h.ignores[0].when.is_none());
    let mut r = room(&h);
    assert!(highlighted(&feed(&mut r, "1", "7", "hey")));
    assert!(feed(&mut r, "2", "7", "bad").is_empty());
    let back = serde_json::to_string(&h.phrases[0]).unwrap();
    assert!(!back.contains("when"), "nothing new written for rules without conditions");
}

/// 200 rules, all with conditions, over 10.000 lines. Run with `--nocapture` for the time.
#[test]
fn many_conditioned_rules_stay_fast() {
    let when = |i: usize| When {
        channels: if i.is_multiple_of(2) { vec!["chan".into(), format!("c{i}")] } else { vec![format!("c{i}")] },
        not_channels: i.is_multiple_of(3),
        categories: if i.is_multiple_of(4) { vec!["Just Chatting".into(), "Minecraft".into()] } else { Vec::new() },
        live: i.is_multiple_of(5).then_some(LiveWhen::Live),
        role: i.is_multiple_of(7).then_some(RoleWhen::Mod),
        burst: if i.is_multiple_of(6) { 3 } else { 0 },
        burst_secs: 10,
        first_only: false,
    };
    let h = Highlights {
        phrases: (0..80).map(|i| word(&format!("word{i}"), when(i))).collect(),
        users: (0..60)
            .map(|i| UserRule { login: format!("u{i}"), when: Some(when(i)), ..UserRule::default() })
            .collect(),
        badges: (0..20)
            .map(|i| BadgeRule { badge: format!("b{i}"), when: Some(when(i)), ..BadgeRule::default() })
            .collect(),
        ignores: (0..40)
            .map(|i| IgnoreRule { pattern: format!("bad{i}"), when: Some(when(i)), ..IgnoreRule::default() })
            .collect(),
        ..Highlights::default()
    };
    let raw: Vec<String> = (0..10_000)
        .map(|i| {
            format!(
                "@id={i};user-id={};tmi-sent-ts={}000 :u{}!u@u PRIVMSG #chan :hello word{} and bad{} there",
                i % 300,
                i / 20,
                i % 300,
                i % 90,
                i % 50
            )
        })
        .collect();
    let lines: Vec<Line> = raw.iter().map(|s| Line::parse(s).unwrap()).collect();
    let run = |h: &Highlights| {
        let mut r = room(h);
        r.set_stream(true, Some("Minecraft"));
        let start = std::time::Instant::now();
        for l in &lines {
            r.handle_line(l, 1_000, &NoEmotes);
            r.take_outcomes();
        }
        start.elapsed()
    };
    let base = run(&Highlights::default());
    let with = run(&h);
    let mut plain = h.clone();
    plain.phrases.iter_mut().for_each(|r| r.when = None);
    plain.users.iter_mut().for_each(|r| r.when = None);
    plain.badges.iter_mut().for_each(|r| r.when = None);
    plain.ignores.iter_mut().for_each(|r| r.when = None);
    let unconditioned = run(&plain);
    println!(
        "V110-C59 10.000 lines: {base:?} without rules, {unconditioned:?} with the 200 rules unconditioned, {with:?} with conditions"
    );
    assert!(with.as_secs_f64() < 10.0, "{with:?}");
}
