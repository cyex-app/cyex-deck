//! V110-F89 IRC line to chat message, notice and state on generated lines.

use crate::build::{Ctx, Incoming, Me, build};
use crate::fuzz::{Rng, byte_prefixes};
use crate::irc::Line;
use crate::model::plain;

const SEEDS: &[&str] = &[
    "@badge-info=subscriber/14;badges=subscriber/12,vip/1;color=#FF0000;display-name=Bob;emotes=25:0-4;id=m1;room-id=1;tmi-sent-ts=1700000000000;user-id=7;bits=100;first-msg=1 :bob!bob@bob.tmi.twitch.tv PRIVMSG #bar :Kappa cheer100 hello",
    "@id=m2;user-id=7 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :\u{1}ACTION waves at cyex\u{1}",
    r"@id=m4;user-id=7;reply-parent-msg-id=p1;reply-parent-user-login=cyex;reply-parent-display-name=Cyex;reply-parent-msg-body=hi\sall;reply-thread-parent-msg-id=t1 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :@cyex hello back",
    "@id=m5;user-id=7;room-id=1;source-room-id=2;source-badges=moderator/1;custom-reward-id=r1;msg-id=gigantified-emote-message;first-msg=1;gifs=0-1|g|https://a.b/c.gif :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :x y",
    "@id=m6;user-id=7;pinned-chat-paid-amount=500;pinned-chat-paid-currency=USD;pinned-chat-paid-exponent=2 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :paid",
    r"@badges=subscriber/12;display-name=Foo;id=n1;login=foo;msg-id=resub;msg-param-cumulative-months=12;msg-param-streak-months=99999999999999999999;system-msg=Foo\ssubscribed\sat\sTier\s1.;tmi-sent-ts=5;user-id=7 :tmi.twitch.tv USERNOTICE #c :love this stream",
    "@display-name=Mod;id=n2;login=mod;msg-id=announcement;msg-param-color=PURPLE;system-msg=;user-id=8 :tmi.twitch.tv USERNOTICE #c :Giveaway at 8",
    r"@id=n3;login=a;msg-id=sharedchatnotice;source-msg-id=raid;room-id=1;source-room-id=2;system-msg=5\sraiders\sfrom\sA\shave\sjoined!;user-id=9 :tmi.twitch.tv USERNOTICE #c",
    r"@id=g0;login=a;msg-id=submysterygift;msg-param-mass-gift-count=5;msg-param-origin-id=777;msg-param-sender-count=-3;system-msg=A\sis\sgifting\s5;user-id=9 :tmi.twitch.tv USERNOTICE #c",
    r"@id=g1;login=a;msg-id=subgift;msg-param-community-gift-id=777;msg-param-origin-id=777;msg-param-months=4294967296;msg-param-recipient-display-name=Bob;msg-param-recipient-user-name=bob;system-msg=A\sgifted;user-id=9 :tmi.twitch.tv USERNOTICE #c",
    r"@id=g3;login=ananonymousgifter;msg-id=anonsubgift;msg-param-recipient-display-name=Bob;msg-param-recipient-user-name=bob;user-id=9 :tmi.twitch.tv USERNOTICE #c",
    r"@display-name=Mod;id=n4;login=mod;msg-id=raid;msg-param-viewerCount=18446744073709551615;msg-param-displayName=X;msg-param-login=x;user-id=8 :tmi.twitch.tv USERNOTICE #c",
    "@msg-id=bitsbadgetier;msg-param-threshold=1000000000000;id=b;login=x;user-id=1 :tmi.twitch.tv USERNOTICE #c :hi",
    "@ban-duration=600;room-id=1;target-user-id=9;tmi-sent-ts=10 :tmi.twitch.tv CLEARCHAT #c :foo",
    "@ban-duration=99999999999999999999;room-id=1;target-user-id=9 :tmi.twitch.tv CLEARCHAT #c :foo",
    "@ban-duration=-5;room-id=1;target-user-id=9 :tmi.twitch.tv CLEARCHAT #c :foo",
    "@room-id=1 :tmi.twitch.tv CLEARCHAT #c",
    "@login=foo;target-msg-id=abc :tmi.twitch.tv CLEARMSG #c :hi",
    "@msg-id=msg_duplicate :tmi.twitch.tv NOTICE #c :Your message was not sent because it is identical",
    "@msg-id=msg_slowmode;msg-param-slow-duration=99999999999999999999 :tmi.twitch.tv NOTICE #c :You are in slow mode",
    "@emote-only=0;followers-only=-1;r9k=0;room-id=12;slow=0;subs-only=0 :tmi.twitch.tv ROOMSTATE #c",
    "@followers-only=99999999999999999999;slow=-1;room-id=12 :tmi.twitch.tv ROOMSTATE #c",
    "@badges=vip/1;color=#FF0000;display-name=Me;id=abc;mod=0 :tmi.twitch.tv USERSTATE #c",
    "@badges=lead_moderator/1;mod=1 :tmi.twitch.tv USERSTATE #c",
    ":a!a@a.tmi.twitch.tv JOIN #chan",
    ":a!a@a.tmi.twitch.tv PART #chan",
];

fn feed(raw: &str, me: Option<&Me>, now: i64) -> Option<Incoming> {
    let line = Line::parse(raw)?;
    let mut colors = Default::default();
    let mut ctx = Ctx::new(me, &mut colors, now);
    build(&line, &mut ctx)
}

#[test]
fn build_never_panics_on_mutated_and_truncated_lines() {
    let me = Me::new("7", "foo", "Foo");
    let mut r = Rng::new(0xB0B);
    let start = std::time::Instant::now();
    let mut cases = 0;
    for s in SEEDS {
        for p in byte_prefixes(s) {
            let _ = feed(&p, Some(&me), 5);
            let _ = feed(&p, None, i64::MAX);
            cases += 1;
        }
        for _ in 0..300 {
            let m = r.mutate(s);
            let _ = feed(&m, Some(&me), 0);
            let _ = feed(&m, None, i64::MIN);
            cases += 1;
        }
    }
    for _ in 0..2000 {
        let _ = feed(&r.text(80), Some(&me), 1);
    }
    assert!(cases > 5000);
    assert!(start.elapsed().as_secs() < 30, "{:?}", start.elapsed());
}

#[test]
fn a_plain_privmsg_keeps_its_words() {
    let me = Me::new("42", "cyex", "Cyex");
    let mut r = Rng::new(0x9A11);
    for _ in 0..3000 {
        let text = if r.chance(50) { r.text(50) } else { r.words(12) };
        // A line never carries its own line end.
        if text.starts_with('\u{1}') || text.ends_with(['\r', '\n']) {
            continue;
        }
        let raw = format!("@id=m;user-id=7 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :{text}");
        if Line::parse(&raw).is_none() {
            continue;
        }
        match feed(&raw, Some(&me), 0) {
            // V110-F94 As shown: without control characters and bidi overrides.
            Some(Incoming::Message(m)) => assert_eq!(plain(&m.body), crate::tokenize::shown(&text), "{raw:?}"),
            other => panic!("{raw:?} -> {other:?}"),
        }
    }
}
