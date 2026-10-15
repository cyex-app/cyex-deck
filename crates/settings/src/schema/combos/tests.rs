use super::*;

const A: &str = "123";

fn item(id: &str) -> Option<ComboItem> {
    Some(ComboItem {
        id: id.into(),
        name: format!("name {id}"),
    })
}

fn save(c: &mut Combos, name: &str) -> Result<Combo, ComboError> {
    match c.change(
        A,
        ComboOp::Save {
            name: name.into(),
            paint: item("p1"),
            badge: item("b1"),
        },
    )? {
        ComboDone::Saved { combo } => Ok(combo),
        other => panic!("{other:?}"),
    }
}

fn names(c: &Combos) -> Vec<&str> {
    c.list(A).iter().map(|c| c.name.as_str()).collect()
}

#[test]
fn names_follow_the_fixture() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../tests/fixtures/combo_names.json")).unwrap();
    assert!(cases.len() >= 14);
    for case in cases {
        let input = case["in"].as_str().unwrap();
        let got = clean_name(input);
        match (case["out"].as_str(), case["err"].as_str()) {
            (Some(out), None) => assert_eq!(got.as_deref(), Ok(out), "{input:?}"),
            (None, Some(err)) => {
                let code = match got {
                    Err(NameError::Empty) => "empty",
                    Err(NameError::TooLong) => "tooLong",
                    Err(NameError::Chars) => "chars",
                    Err(NameError::Word(_)) => "word",
                    Ok(n) => panic!("{input:?} gave {n:?}"),
                };
                assert_eq!(code, err, "{input:?}");
            }
            _ => panic!("bad case {case}"),
        }
    }
}

#[test]
fn messages_are_the_ones_users_see() {
    assert_eq!(ComboError::Missing.to_string(), "That combo is gone.");
    assert_eq!(
        ComboError::Full.to_string(),
        "You have 50 combos. Delete one first."
    );
    assert_eq!(
        ComboError::Taken("Gold".into()).to_string(),
        "You have a combo called “Gold” already."
    );
    assert_eq!(
        NameError::Word("list".into()).to_string(),
        "“list” is part of /combo. Pick another name."
    );
}

#[test]
fn saves_up_to_fifty_then_full() {
    let mut c = Combos::default();
    for i in 0..MAX_COMBOS {
        save(&mut c, &format!("c{i}")).unwrap();
    }
    assert_eq!(save(&mut c, "one more"), Err(ComboError::Full));
    assert_eq!(c.list(A).len(), 50);
}

#[test]
fn names_are_unique_across_cases_and_per_account() {
    let mut c = Combos::default();
    save(&mut c, "Gold").unwrap();
    assert_eq!(save(&mut c, " gold "), Err(ComboError::Taken("gold".into())));
    c.change(
        "456",
        ComboOp::Save {
            name: "gold".into(),
            paint: None,
            badge: None,
        },
    )
    .unwrap();
    assert!(c.find(A, "GOLD").is_some());
}

#[test]
fn rename_to_another_case_of_itself_works() {
    let mut c = Combos::default();
    let a = save(&mut c, "gold").unwrap();
    save(&mut c, "silver").unwrap();
    c.change(
        A,
        ComboOp::Rename {
            id: a.id.clone(),
            name: "Gold".into(),
        },
    )
    .unwrap();
    assert_eq!(names(&c), ["Gold", "silver"]);
    let r = c.change(
        A,
        ComboOp::Rename {
            id: a.id,
            name: "SILVER".into(),
        },
    );
    assert_eq!(r.unwrap_err(), ComboError::Taken("SILVER".into()));
}

#[test]
fn edit_keeps_id_and_name_and_stores_none() {
    let mut c = Combos::default();
    let a = save(&mut c, "gold").unwrap();
    let done = c
        .change(
            A,
            ComboOp::Edit {
                id: a.id.clone(),
                paint: None,
                badge: item("b2"),
            },
        )
        .unwrap();
    let ComboDone::Changed { combo } = done else {
        panic!()
    };
    assert_eq!((combo.id.as_str(), combo.name.as_str()), (a.id.as_str(), "gold"));
    assert_eq!(c.list(A)[0].paint, None);
    assert_eq!(c.list(A)[0].badge, item("b2"));
    c.change(
        A,
        ComboOp::Edit {
            id: a.id,
            paint: item("p2"),
            badge: None,
        },
    )
    .unwrap();
    assert_eq!(c.list(A)[0].badge, None);
}

#[test]
fn duplicate_names_and_fits() {
    let mut c = Combos::default();
    let a = save(&mut c, "gold").unwrap();
    save(&mut c, "last").unwrap();
    for _ in 0..2 {
        c.change(A, ComboOp::Duplicate { id: a.id.clone() }).unwrap();
    }
    assert_eq!(names(&c), ["gold", "gold copy 2", "gold copy", "last"]);
    let long = "x".repeat(32);
    let b = save(&mut c, &long).unwrap();
    let ComboDone::Saved { combo } = c.change(A, ComboOp::Duplicate { id: b.id.clone() }).unwrap() else {
        panic!()
    };
    assert_eq!(combo.name, format!("{} copy", "x".repeat(27)));
    assert_eq!(combo.name.chars().count(), 32);
    let ComboDone::Saved { combo } = c.change(A, ComboOp::Duplicate { id: b.id }).unwrap() else {
        panic!()
    };
    assert_eq!(combo.name, format!("{} copy 2", "x".repeat(25)));
    assert!(combo.name.chars().count() <= 32);
}

#[test]
fn delete_then_restore_puts_it_back() {
    let mut c = Combos::default();
    save(&mut c, "a").unwrap();
    let b = save(&mut c, "b").unwrap();
    save(&mut c, "c").unwrap();
    let ComboDone::Deleted { combo, at } = c.change(A, ComboOp::Delete { id: b.id.clone() }).unwrap() else {
        panic!()
    };
    assert_eq!((at, names(&c)), (1, vec!["a", "c"]));
    c.change(A, ComboOp::Restore { combo, at }).unwrap();
    assert_eq!(names(&c), ["a", "b", "c"]);
    assert_eq!(c.list(A)[1].id, b.id);
}

#[test]
fn restore_over_a_reused_name_is_taken() {
    let mut c = Combos::default();
    let a = save(&mut c, "a").unwrap();
    let ComboDone::Deleted { combo, at } = c.change(A, ComboOp::Delete { id: a.id }).unwrap() else {
        panic!()
    };
    save(&mut c, "A").unwrap();
    let r = c.change(A, ComboOp::Restore { combo, at });
    assert_eq!(r.unwrap_err(), ComboError::Taken("a".into()));
}

#[test]
fn unknown_ids_are_missing() {
    let mut c = Combos::default();
    for op in [
        ComboOp::Rename {
            id: "x".into(),
            name: "n".into(),
        },
        ComboOp::Edit {
            id: "x".into(),
            paint: None,
            badge: None,
        },
        ComboOp::Duplicate { id: "x".into() },
        ComboOp::Delete { id: "x".into() },
    ] {
        assert_eq!(c.change(A, op).unwrap_err(), ComboError::Missing);
    }
    assert!(c.accounts.is_empty());
}

#[test]
fn sanitize_trims_the_pile() {
    let combo = |id: &str, name: &str| Combo {
        id: id.into(),
        name: name.into(),
        ..Combo::default()
    };
    let mut list: Vec<Combo> = (0..60)
        .map(|i| combo(&format!("i{i}"), &format!("n{i}")))
        .collect();
    list.push(combo("z", "N1"));
    let mut c = Combos {
        accounts: [
            (A.to_owned(), list),
            ("abc".into(), vec![combo("a", "a")]),
            ("9".into(), vec![]),
        ]
        .into(),
    };
    c.sanitize();
    assert_eq!(c.accounts.keys().collect::<Vec<_>>(), [A]);
    assert_eq!(c.list(A).len(), 50);

    let mut c = Combos {
        accounts: [(
            A.to_owned(),
            vec![
                combo("", "one"),
                combo("dup", "two"),
                combo("dup", "three"),
                combo("ok", "two"),
                combo("bad", "a/b"),
                Combo {
                    paint: item("not ok!"),
                    badge: Some(ComboItem {
                        id: "b1".into(),
                        name: "y".repeat(120),
                    }),
                    ..combo("k", " four  4 ")
                },
            ],
        )]
        .into(),
    };
    c.sanitize();
    assert_eq!(names(&c), ["one", "two", "three", "four 4"]);
    let ids: HashSet<_> = c.list(A).iter().map(|c| &c.id).collect();
    assert_eq!(ids.len(), 4);
    assert!(c.list(A).iter().all(|c| !c.id.is_empty()));
    let last = &c.list(A)[3];
    assert_eq!(last.paint, None);
    assert_eq!(last.badge.as_ref().unwrap().name.chars().count(), 100);
}

#[test]
fn missing_file_is_the_default_and_holds_no_secrets() {
    let c: Combos = serde_json::from_str("{}").unwrap();
    assert_eq!(c, Combos::default());
    let mut c = Combos::default();
    save(&mut c, "gold").unwrap();
    let v = serde_json::to_value(&c).unwrap();
    let combo = &v["accounts"][A][0];
    let mut keys: Vec<_> = combo.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["badge", "id", "name", "paint"]);
    assert_eq!(v.as_object().unwrap().keys().collect::<Vec<_>>(), ["accounts"]);
}
