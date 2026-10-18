use super::*;
use crate::tokenize::NoEmotes;
use cyex_settings::schema::{Alert, Highlights, PhraseRule};

// V106-33 A rule with only the pop-up on still reaches the app.
#[test]
fn popup_only_match_is_queued() {
    let h = Highlights {
        phrases: vec![PhraseRule {
            pattern: "pasta".into(),
            alert: Alert {
                notify: true,
                ..Alert::default()
            },
            ..PhraseRule::default()
        }],
        ..Highlights::default()
    };
    let mut r = Room::new("chan", Some(Me::new("42", "me", "Me")));
    r.set_rules(Arc::new(Rules::new(&h)));
    let raw = "@id=1;user-id=7;tmi-sent-ts=1000 :u7!u@u PRIVMSG #chan :pasta time";
    r.handle_line(&Line::parse(raw).unwrap(), 1_000, &NoEmotes);
    let out = r.take_outcomes();
    assert_eq!(out.len(), 1);
    let o = &out[0].1;
    assert!(o.notify && !o.sound && !o.flash && o.routes.is_empty());
}
