//! Parse + tokenize speed (target ≤ 20 µs per message, DECISIONS.md). Run with
//! `cargo test -p cyex-chat --release --test bench -- --ignored --nocapture`.

use std::time::Instant;

use cyex_chat::Room;
use cyex_chat::build::Me;
use cyex_chat::irc::Line;
use cyex_chat::tokenize::NoEmotes;

const LINES: [&str; 4] = [
    "@badge-info=subscriber/26;badges=subscriber/24,premium/1;color=#1E90FF;display-name=SomeViewer;emotes=25:6-10,32-36/1902:12-16;first-msg=0;id=0b6d3a1e-1111-2222-3333-444455556666;mod=0;returning-chatter=0;room-id=71092938;subscriber=1;tmi-sent-ts=1791493255184;turbo=0;user-id=123456789;user-type= :someviewer!someviewer@someviewer.tmi.twitch.tv PRIVMSG #xqc :hello Kappa Keepo what a play, check twitch.tv/xqc Kappa @Other",
    "@badge-info=;badges=;color=;display-name=Plain;emotes=;id=1;room-id=1;tmi-sent-ts=1;user-id=2 :plain!plain@plain.tmi.twitch.tv PRIVMSG #xqc :OOOO REAL",
    r"@badge-info=subscriber/10;badges=subscriber/9;color=#FF0000;display-name=Gifter;id=2;login=gifter;msg-id=resub;msg-param-cumulative-months=10;msg-param-sub-plan=1000;room-id=1;system-msg=Gifter\ssubscribed\sat\sTier\s1.\sThey've\ssubscribed\sfor\s10\smonths!;tmi-sent-ts=1;user-id=3 :tmi.twitch.tv USERNOTICE #xqc :ten months 👋🏽 LUL",
    r"@badge-info=;badges=vip/1;color=#00FF7F;display-name=Replier;id=3;reply-parent-display-name=SomeViewer;reply-parent-msg-body=hello\sKappa;reply-parent-msg-id=0b6d;reply-parent-user-id=123456789;reply-parent-user-login=someviewer;reply-thread-parent-msg-id=0b6d;room-id=1;tmi-sent-ts=1;user-id=4;vip=1 :replier!replier@replier.tmi.twitch.tv PRIVMSG #xqc :@SomeViewer agreed, https://example.com/a?b=c is the link",
];

#[test]
#[ignore = "benchmark"]
fn parse_and_tokenize_speed() {
    let mut room = Room::new("xqc", Some(Me::new("42", "me", "Me")));
    let n = 50_000;
    let start = Instant::now();
    for i in 0..n {
        let raw = LINES[i % LINES.len()];
        let line = Line::parse(raw).unwrap();
        let events = room.handle_line(&line, 0, &NoEmotes);
        assert!(!events.is_empty());
    }
    let per = start.elapsed() / n as u32;
    println!("parse + tokenize + room: {per:?} per message");
    assert!(per.as_micros() <= 20, "{per:?} per message");
}
