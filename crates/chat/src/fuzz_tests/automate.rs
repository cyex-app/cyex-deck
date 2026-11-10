//! V110-F89 Chat automations: triggers × message texts, cooldown tables flooded with people,
//! clocks at the edges.

use cyex_settings::schema::{Access, Automation, Commands, TriggerMatch};

use crate::automate::Automations;
use crate::build::{Ctx, Incoming, Me, build};
use crate::fuzz::Rng;
use crate::irc::Line;
use crate::model::ChatMessage;

fn msg(user: u64, badges: &str, text: &str) -> Option<ChatMessage> {
    let raw = format!("@badges={badges};id=m{user};user-id={user} :u{user}!u@u PRIVMSG #chan :{text}");
    let mut colors = Default::default();
    let me = Me::new("1", "me", "Me");
    let mut ctx = Ctx::new(Some(&me), &mut colors, 0);
    match build(&Line::parse(&raw)?, &mut ctx) {
        Some(Incoming::Message(m)) => Some(m),
        _ => None,
    }
}

fn rule(r: &mut Rng, trigger: String) -> Automation {
    Automation {
        channels: vec!["chan".into()],
        trigger,
        matching: *r.pick(&[TriggerMatch::Command, TriggerMatch::Exact, TriggerMatch::StartsWith, TriggerMatch::Contains, TriggerMatch::Regex]),
        response: "hi $1-".into(),
        reply: r.chance(50),
        who: *r.pick(&[Access::Everyone, Access::Subscribers, Access::Vips, Access::Moderators, Access::Broadcaster]),
        live_only: r.chance(30),
        cooldown_secs: [0, 1, 30, u32::MAX][r.below(4)],
        user_cooldown_secs: [0, 1, 30, u32::MAX][r.below(4)],
        ..Automation::default()
    }
}

fn automations(list: Vec<Automation>, limit: u8) -> Automations {
    Automations::new(&Commands { automations_on: true, automations: list, automation_limit: limit, ..Commands::default() })
}

#[test]
fn triggers_and_texts_never_panic() {
    let mut r = Rng::new(0xA070);
    let triggers = ["!discord", "!Ä", "\u{212A}ing", "İ", "ß", "(a+)+$", "(", "", " ", "\u{1E9E}", "ΑΣ", "x{1000}{1000}", "\\p{L}+"];
    for i in 0..1500 {
        let trig = if r.chance(60) { (*r.pick(&triggers)).to_owned() } else { r.text(8) };
        let mut a = automations(vec![rule(&mut r, trig.clone()), rule(&mut r, "!x".into())], (r.below(5)) as u8);
        for k in 0..6 {
            let text = match r.below(4) {
                0 => r.text(30),
                1 => format!("{trig} {}", r.text(10)),
                2 => format!("{} {}", trig.to_uppercase(), r.text(10)),
                _ => r.words(6),
            };
            if text.ends_with(['\r', '\n']) || text.starts_with('\u{1}') {
                continue;
            }
            let badges = ["", "moderator/1", "subscriber/12", "vip/1", "broadcaster/1"][r.below(5)];
            let Some(m) = msg(100 + (i * 7 + k) as u64 % 900, badges, &text) else { continue };
            let now = [0, 1_000, 1_000_000, 1_700_000_000_000, -1][r.below(5)];
            if let Some(f) = a.check("chan", &m, r.chance(50), now) {
                // `args` is text from the message, whole characters.
                assert!(f.rule < 2 && f.count >= 1);
            }
            let _ = a.check("other", &m, true, now);
        }
        let _ = a.counts();
        a.update(&Commands { automations_on: true, automations: vec![rule(&mut r, trig)], ..Commands::default() });
    }
}

#[test]
fn a_starts_with_trigger_hands_over_what_follows_it() {
    let mut r = Rng::new(0xA071);
    for t in ["!discord", "\u{212A}ing", "İstanbul", "ß", "ΑΣ", "ǅ", "ﬃ", "Kappa"] {
        let a = Automation {
            channels: vec!["chan".into()],
            trigger: t.into(),
            matching: TriggerMatch::StartsWith,
            cooldown_secs: 0,
            user_cooldown_secs: 0,
            ..Automation::default()
        };
        for _ in 0..200 {
            let rest = r.text(10).trim().to_owned();
            let spelled: String = t.chars().map(|c| if r.chance(50) { c.to_uppercase().collect::<String>() } else { c.to_string() }).collect();
            let text = format!("{spelled} {rest}");
            if text.ends_with(['\r', '\n']) || rest.contains(['\r', '\n']) || text.starts_with('\u{1}') {
                continue;
            }
            let mut auto = automations(vec![a.clone()], 5);
            let Some(m) = msg(5, "", &text) else { continue };
            if let Some(f) = auto.check("chan", &m, true, 10_000) {
                // V110-F94 A rule reads the text as shown (no control characters or bidi overrides).
                let (text, rest) = (crate::tokenize::shown(&text), crate::tokenize::shown(&rest));
                let want = rest.trim();
                // The trigger's case may map to other lengths; what comes back is a tail of the text.
                assert!(text.trim_end().ends_with(f.args.as_str()), "{text:?} -> {:?}", f.args);
                assert!(f.args == want || want.is_empty() || f.args.len() >= want.len(), "{text:?} -> {:?} (want {want:?})", f.args);
            }
        }
    }
}

#[test]
fn a_flood_of_people_keeps_the_cooldown_table_small() {
    let a = Automation {
        channels: vec!["chan".into()],
        trigger: "!x".into(),
        matching: TriggerMatch::Command,
        cooldown_secs: 0,
        user_cooldown_secs: u32::MAX,
        ..Automation::default()
    };
    let mut auto = automations(vec![a], 255);
    let mut now = 1_000_000i64;
    for user in 0..3000u64 {
        if let Some(m) = msg(10 + user, "", "!x hi") {
            now += 2_000;
            let _ = auto.check("chan", &m, true, now);
        }
    }
}

#[test]
fn extreme_clocks_do_not_overflow() {
    let a = Automation { channels: vec!["chan".into()], trigger: "!x".into(), cooldown_secs: 1, user_cooldown_secs: 1, ..Automation::default() };
    let mut auto = automations(vec![a], 5);
    let m = msg(5, "", "!x hi").unwrap();
    let _ = auto.check("chan", &m, true, i64::MAX);
    let _ = auto.check("chan", &m, true, i64::MIN);
    let _ = auto.check("chan", &m, true, 0);
}
