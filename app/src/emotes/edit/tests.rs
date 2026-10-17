use std::time::{Duration, Instant};

use cyex_chat::model::Provider;
use cyex_emotes::Emote;
use cyex_emotes::seventv::account::sets::{Grant, Reachable, SetBrief, SetKind};
use cyex_emotes::seventv::events::SetChange;

use super::echo::Echoes;
use super::ops::*;
use super::options::{Facts, Viewed, options};
use super::reach::{Reach, Target, build, set_ids};

fn grant(id: &str, name: Option<&str>, set: Option<&str>, accepted: bool, manage: bool) -> Grant {
    Grant {
        user_id: id.into(),
        name: name.map(Into::into),
        set_id: set.map(Into::into),
        accepted,
        manage,
    }
}

fn set(id: &str, kind: SetKind, entries: &[(&str, &str)]) -> SetBrief {
    SetBrief {
        id: id.into(),
        kind,
        capacity: Some(600),
        owner_id: None,
        entries: entries
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
    }
}

fn me(name: Option<&str>, grants: Vec<Grant>) -> Reachable {
    Reachable {
        user_id: "ME".into(),
        name: name.map(Into::into),
        set_id: Some("S_ME".into()),
        grants,
    }
}

fn target(owner: &str, channel: &str, own: bool, set: &str, entries: &[(&str, &str)]) -> Target {
    Target {
        owner_id: owner.into(),
        channel: channel.into(),
        own,
        set_id: set.into(),
        capacity: Some(600),
        entries: entries
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
    }
}

#[test]
fn build_keeps_own_and_accepted_editable_normal_sets() {
    let m = me(
        Some("cyexbot"),
        vec![
            grant("ED", Some("mockedit"), Some("S_ED"), true, true),
            grant("PE", Some("pending"), Some("S_PE"), false, true),
            grant("NO", Some("noperm"), Some("S_NO"), true, false),
            grant("PS", Some("personal"), Some("S_PS"), true, true),
            grant("NA", Some("noactive"), None, true, true),
            grant("ME", Some("self"), Some("S_SELF"), true, true),
            grant("ID", None, Some("S_ID"), true, true),
        ],
    );
    assert_eq!(set_ids(&m), ["S_ME", "S_ED", "S_PS", "S_ID"]);
    let sets = vec![
        set("S_ED", SetKind::Normal, &[("1", "a")]),
        set("S_ME", SetKind::Normal, &[]),
        set("S_PE", SetKind::Normal, &[]),
        set("S_NO", SetKind::Normal, &[]),
        set("S_PS", SetKind::Personal, &[]),
        set("S_SELF", SetKind::Normal, &[]),
        set("S_ID", SetKind::Normal, &[]),
    ];
    let r = build(m, sets);
    assert_eq!(r.me, "cyexbot");
    let got: Vec<(&str, &str, bool)> = r
        .targets
        .iter()
        .map(|t| (t.channel.as_str(), t.set_id.as_str(), t.own))
        .collect();
    assert_eq!(
        got,
        [
            ("cyexbot", "S_ME", true),
            ("mockedit", "S_ED", false),
            ("ID", "S_ID", false)
        ]
    );
    assert_eq!(r.targets[1].entries, [("1".to_string(), "a".to_string())]);
}

#[test]
fn build_names_me_you_without_a_name_and_caps_ids() {
    let r = build(me(None, vec![]), vec![]);
    assert_eq!(r.me, "You");
    assert!(r.targets.is_empty());
    let many: Vec<Grant> = (0..60)
        .map(|n| grant(&format!("U{n}"), None, Some(&format!("S{n}")), true, true))
        .collect();
    assert_eq!(set_ids(&me(None, many)).len(), 50);
}

#[test]
fn target_checks_and_applies() {
    let mut t = target("O", "o", false, "S", &[("A", "a1"), ("A", "a2"), ("B", "b")]);
    assert!(t.has("A") && !t.has("C"));
    assert!(t.taken("b") && !t.taken("B"));
    assert!(!t.full());
    t.capacity = Some(3);
    assert!(t.full());
    let add = EditOp::Add {
        set_id: "S".into(),
        emote_id: "C".into(),
        alias: "c".into(),
    };
    t.apply(&add);
    t.apply(&add);
    assert_eq!(t.entries.len(), 4);
    t.apply(&EditOp::Remove {
        set_id: "S".into(),
        emote_id: "A".into(),
        alias: "a1".into(),
    });
    t.apply(&EditOp::Rename {
        set_id: "S".into(),
        emote_id: "B".into(),
        from: "b".into(),
        to: "bee".into(),
    });
    let names: Vec<&str> = t.entries.iter().map(|(_, a)| a.as_str()).collect();
    assert_eq!(names, ["a2", "bee", "c"]);
}

#[test]
fn target_follows_7tv_events() {
    let mut t = target("O", "o", false, "S", &[("A", "a"), ("B", "b")]);
    let c = SetChange {
        set_id: "S".into(),
        actor: None,
        added: vec![Emote::new(Provider::Seventv, "C", "c")],
        removed: vec![("A".into(), "lagging".into())],
        renamed: vec![("B".into(), "b".into(), Emote::new(Provider::Seventv, "B", "bee"))],
    };
    t.apply_change(&c);
    assert_eq!(
        t.entries,
        [
            ("B".to_string(), "bee".to_string()),
            ("C".to_string(), "c".to_string())
        ]
    );
}

fn reach() -> Reach {
    Reach {
        me: "cyexbot".into(),
        targets: vec![
            target("ME", "cyexbot", true, "S_ME", &[("X", "x")]),
            target("ZED", "zed", false, "S_Z", &[]),
            target("AB", "Abby", false, "S_AB", &[]),
            target("VW", "viewed", false, "S_VW", &[("E", "emo")]),
        ],
    }
}

fn facts<'a>(id: &'a str, private: bool, owner: Option<&'a str>) -> Facts<'a> {
    Facts {
        seventv: true,
        emote_id: id,
        name: "emo",
        own_name: "emoOwn",
        private,
        owner_id: owner,
    }
}

const STRANGER: Viewed = Viewed {
    owner_id: Some("STRANGER"),
    set_id: Some("S_STR"),
    in_set: true,
};

#[test]
fn options_order_and_here() {
    let r = reach();
    let v = Viewed {
        owner_id: Some("VW"),
        set_id: Some("S_VW"),
        in_set: true,
    };
    let o = options(Reachability::Ready, Some(&r), &v, &facts("E", false, None));
    assert_eq!(o.own_name, "emoOwn");
    let add = o.add.unwrap();
    let order: Vec<&str> = add.targets.iter().map(|t| t.channel.as_str()).collect();
    assert_eq!(order, ["viewed", "cyexbot", "Abby", "zed"]);
    assert!(add.targets[0].viewed && add.targets[1].own);
    assert_eq!(add.targets[0].blocked, Some(Blocked::Already));
    assert_eq!(add.blocked, None);
    assert_eq!(
        o.here,
        Some(HereEdit {
            set_id: "S_VW".into(),
            channel: "viewed".into(),
            alias: "emo".into()
        })
    );
    // Not in this chat's set, or a set that isn't the target's: no rename or remove.
    let v = Viewed { in_set: false, ..v };
    assert_eq!(
        options(Reachability::Ready, Some(&r), &v, &facts("E", false, None)).here,
        None
    );
    let v = Viewed {
        owner_id: Some("VW"),
        set_id: Some("OLD"),
        in_set: true,
    };
    assert_eq!(
        options(Reachability::Ready, Some(&r), &v, &facts("E", false, None)).here,
        None
    );
    // A channel the account can't edit.
    let o = options(Reachability::Ready, Some(&r), &STRANGER, &facts("E", false, None));
    assert_eq!(o.here, None);
    assert!(o.add.unwrap().targets.iter().all(|t| !t.viewed));
}

#[test]
fn options_blocked_reasons() {
    let r = reach();
    // Private: only the owner's channel can take it.
    let o = options(
        Reachability::Ready,
        Some(&r),
        &STRANGER,
        &facts("P", true, Some("ZED")),
    );
    let add = o.add.unwrap();
    assert_eq!(add.blocked, None);
    for t in &add.targets {
        let want = if t.channel == "zed" {
            None
        } else {
            Some(Blocked::Private)
        };
        assert_eq!(t.blocked, want, "{}", t.channel);
    }
    let o = options(
        Reachability::Ready,
        Some(&r),
        &STRANGER,
        &facts("P", true, Some("OTHER")),
    );
    assert_eq!(o.add.unwrap().blocked, Some(AddBlocked::Private));
    // Already everywhere.
    let mut all = reach();
    for t in &mut all.targets {
        t.entries.push(("Q".into(), "q".into()));
    }
    let o = options(
        Reachability::Ready,
        Some(&all),
        &STRANGER,
        &facts("Q", false, None),
    );
    assert_eq!(o.add.unwrap().blocked, Some(AddBlocked::Already));
    // Mixed: already, full.
    let mut mixed = reach();
    for t in &mut mixed.targets {
        if t.has("X") {
            continue;
        }
        t.capacity = Some(t.entries.len() as u32);
    }
    let o = options(
        Reachability::Ready,
        Some(&mixed),
        &STRANGER,
        &facts("X", false, None),
    );
    let add = o.add.unwrap();
    assert_eq!(add.blocked, Some(AddBlocked::Nowhere));
    assert!(add.targets.iter().any(|t| t.blocked == Some(Blocked::Full)));
}

#[test]
fn options_offer_nothing_when_not_ready_or_not_7tv() {
    let r = reach();
    let v = Viewed {
        owner_id: Some("VW"),
        set_id: Some("S_VW"),
        in_set: true,
    };
    for s in [Reachability::Loading, Reachability::NoConsent, Reachability::Down] {
        let o = options(s, Some(&r), &v, &facts("E", false, None));
        assert_eq!((o.add, o.here, o.state), (None, None, s));
    }
    let f = Facts {
        seventv: false,
        ..facts("E", false, None)
    };
    let o = options(Reachability::Ready, Some(&r), &v, &f);
    assert_eq!((o.add, o.here), (None, None));
    let o = options(Reachability::Ready, None, &v, &facts("E", false, None));
    assert_eq!((o.add, o.here), (None, None));
    let empty = Reach {
        me: "You".into(),
        targets: vec![],
    };
    let o = options(Reachability::Ready, Some(&empty), &v, &facts("E", false, None));
    assert_eq!((o.add, o.here), (None, None));
}

#[test]
fn undo_round_trips() {
    let ops = [
        EditOp::Add {
            set_id: "S".into(),
            emote_id: "E".into(),
            alias: "a".into(),
        },
        EditOp::Remove {
            set_id: "S".into(),
            emote_id: "E".into(),
            alias: "a".into(),
        },
        EditOp::Rename {
            set_id: "S".into(),
            emote_id: "E".into(),
            from: "a".into(),
            to: "b".into(),
        },
    ];
    for op in &ops {
        assert_ne!(op.undo(), *op);
        assert_eq!(op.undo().undo(), *op);
    }
    assert_eq!(ops[0].undo(), ops[1]);
}

#[test]
fn serde_shapes() {
    let v = serde_json::to_value(EditFail::RateLimited { seconds: 12 }).unwrap();
    assert_eq!(v, serde_json::json!({"type": "rateLimited", "seconds": 12}));
    assert_eq!(
        serde_json::to_value(EditFail::TestRun).unwrap(),
        serde_json::json!({"type": "testRun"})
    );
    let op = EditOp::Rename {
        set_id: "S".into(),
        emote_id: "E".into(),
        from: "a".into(),
        to: "b".into(),
    };
    let v = serde_json::json!({"type": "rename", "setId": "S", "emoteId": "E", "from": "a", "to": "b"});
    assert_eq!(serde_json::to_value(&op).unwrap(), v);
    assert_eq!(serde_json::from_value::<EditOp>(v).unwrap(), op);
    let add: EditOp = serde_json::from_value(
        serde_json::json!({"type": "add", "setId": "S", "emoteId": "E", "alias": "x"}),
    )
    .unwrap();
    assert_eq!(
        add,
        EditOp::Add {
            set_id: "S".into(),
            emote_id: "E".into(),
            alias: "x".into()
        }
    );
    assert_eq!(
        serde_json::to_value(Reachability::SignInAgain).unwrap(),
        "signInAgain"
    );
    assert_eq!(serde_json::to_value(Blocked::Already).unwrap(), "already");
    assert_eq!(serde_json::to_value(NameCheck::Same).unwrap(), "same");
    let t = AddTarget {
        set_id: "S".into(),
        channel: "c".into(),
        own: true,
        viewed: false,
        blocked: None,
    };
    let v = serde_json::to_value(t).unwrap();
    assert_eq!(v["setId"], "S");
}

// C1 echo: one line, not two.

fn add(id: &str, alias: &str) -> EditOp {
    EditOp::Add {
        set_id: "S".into(),
        emote_id: id.into(),
        alias: alias.into(),
    }
}

fn added(set: &str, id: &str, name: &str) -> SetChange {
    SetChange {
        set_id: set.into(),
        actor: Some("cyexbot".into()),
        added: vec![Emote::new(Provider::Seventv, id, name)],
        ..SetChange::default()
    }
}

#[test]
fn echo_after_settle_is_filtered_once() {
    let t0 = Instant::now();
    let mut e = Echoes::default();
    let op = add("E", "x");
    e.expect(&op);
    e.settle(&op, t0);
    let c = added("S", "E", "x");
    assert!(e.filter(&c, t0 + Duration::from_secs(10)).added.is_empty());
    assert_eq!(e.filter(&c, t0 + Duration::from_secs(11)).added.len(), 1);
}

#[test]
fn echo_while_pending_then_settle_clears() {
    let t0 = Instant::now();
    let mut e = Echoes::default();
    let op = add("E", "x");
    e.expect(&op);
    let c = added("S", "E", "x");
    assert!(e.filter(&c, t0).added.is_empty());
    e.settle(&op, t0);
    assert_eq!(e.filter(&c, t0).added.len(), 1);
    assert!(!e.forget(&op));
}

#[test]
fn forget_says_whether_seen() {
    let t0 = Instant::now();
    let mut e = Echoes::default();
    let op = add("E", "x");
    e.expect(&op);
    e.filter(&added("S", "E", "x"), t0);
    assert!(e.forget(&op));
    e.expect(&op);
    assert!(!e.forget(&op));
}

#[test]
fn echo_too_late_prints() {
    let t0 = Instant::now();
    let mut e = Echoes::default();
    let op = add("E", "x");
    e.expect(&op);
    e.settle(&op, t0);
    let c = added("S", "E", "x");
    assert_eq!(e.filter(&c, t0 + Duration::from_secs(31)).added.len(), 1);
}

#[test]
fn echo_filters_only_our_part() {
    let t0 = Instant::now();
    let mut e = Echoes::default();
    let op = add("E", "x");
    e.expect(&op);
    e.settle(&op, t0);
    let mut c = added("S", "E", "x");
    c.removed.push(("R".into(), "r".into()));
    let left = e.filter(&c, t0);
    assert!(left.added.is_empty());
    assert_eq!(left.removed, [("R".to_string(), "r".to_string())]);
    assert_eq!(left.actor.as_deref(), Some("cyexbot"));
    // Another set: not ours.
    e.expect(&op);
    e.settle(&op, t0);
    assert_eq!(e.filter(&added("OTHER", "E", "x"), t0).added.len(), 1);
}

#[test]
fn echo_remove_matches_by_id_and_rename_by_new_name() {
    let t0 = Instant::now();
    let mut e = Echoes::default();
    let rm = EditOp::Remove {
        set_id: "S".into(),
        emote_id: "E".into(),
        alias: "x".into(),
    };
    e.expect(&rm);
    e.settle(&rm, t0);
    let c = SetChange {
        set_id: "S".into(),
        removed: vec![("E".into(), "lagging".into())],
        ..SetChange::default()
    };
    assert!(e.filter(&c, t0).removed.is_empty());
    let rn = EditOp::Rename {
        set_id: "S".into(),
        emote_id: "E".into(),
        from: "a".into(),
        to: "b".into(),
    };
    e.expect(&rn);
    e.settle(&rn, t0);
    let wrong = SetChange {
        set_id: "S".into(),
        renamed: vec![("E".into(), "a".into(), Emote::new(Provider::Seventv, "E", "c"))],
        ..SetChange::default()
    };
    assert_eq!(e.filter(&wrong, t0).renamed.len(), 1);
    let right = SetChange {
        set_id: "S".into(),
        renamed: vec![("E".into(), "a".into(), Emote::new(Provider::Seventv, "E", "b"))],
        ..SetChange::default()
    };
    assert!(e.filter(&right, t0).renamed.is_empty());
}

#[test]
fn no_event_means_nothing_to_filter() {
    let t0 = Instant::now();
    let mut e = Echoes::default();
    let op = add("E", "x");
    e.expect(&op);
    e.settle(&op, t0);
    // The line came from `apply_local`; the entry simply runs out.
    let other = added("S", "F", "y");
    assert_eq!(e.filter(&other, t0 + Duration::from_secs(31)).added.len(), 1);
    assert!(!e.forget(&op));
}
