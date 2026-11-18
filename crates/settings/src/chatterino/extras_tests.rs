// V111-60 Chatterino's settings.json beyond highlights: ignores, never highlight, muted
// chats, filters, nicknames.

use super::*;

const FILE: &str = include_str!("../../tests/fixtures/chatterino/settings-extras.json");

fn regex(p: &str) -> Result<String, String> {
    if p.starts_with('(') && !p.contains(')') {
        Err("Missing ).".into())
    } else {
        Ok(format!("R:{p}"))
    }
}

fn filter(e: &str) -> Result<(), String> {
    if e.ends_with("&&") { Err("Ends too early.".into()) } else { Ok(()) }
}

fn found() -> Extras {
    parse(FILE, regex, filter).expect("parses")
}

fn reasons(e: &Extras) -> Vec<(&str, &str)> {
    e.skipped.iter().map(|s| (s.pattern.as_str(), s.reason.as_str())).collect()
}

#[test]
fn reads_each_kind_like_chatterino_stores_it() {
    let f = found();
    let ig: Vec<(&str, bool, bool, &str, bool)> = f
        .ignores
        .iter()
        .map(|r| (r.pattern.as_str(), r.regex, r.block, r.replace.as_str(), r.case_sensitive))
        .collect();
    assert_eq!(
        ig,
        [
            ("spoiler", false, true, "***", false),
            ("Kappa", false, false, "K", true),
            ("R:(foo|bar)\\d+", true, false, "\\1", false),
        ]
    );
    assert_eq!(f.never, ["nightbot", "streamelements"]);
    assert_eq!(f.muted, ["forsen", "xqc"]);
    let filters: Vec<(&str, &str, &str)> =
        f.filters.iter().map(|x| (x.id.as_str(), x.name.as_str(), x.expr.as_str())).collect();
    assert_eq!(
        filters,
        [
            ("1b2c3d4e", "Subs only", "author.subbed"),
            ("9a8b7c6d", "From Chatterino", "message.content contains \"hello\""),
        ]
    );
    let nicks: Vec<(&str, &str)> = f.nicknames.iter().map(|n| (n.login.as_str(), n.nickname.as_str())).collect();
    assert_eq!(
        nicks,
        [
            ("pajlada", "Pajbot dad"),
            ("ghost_user_404", "Gone"),
            ("longname", "a nickname that is much l"),
        ]
    );
    let r = reasons(&f);
    assert!(r.contains(&("(broken", "Ignore: Missing ).")));
    assert!(r.contains(&(".*bot$", "Never highlight takes one name per row, no regex.")));
    assert!(r.contains(&("not a channel", "Muted chat: not a channel name.")));
    assert!(r.contains(&("Broken", "Filter: Ends too early.")));
    assert!(r.iter().any(|(p, why)| *p == "^mod.*" && why.starts_with("Nickname: regex")));
    assert!(r.contains(&("two words", "Nickname: not a Twitch name.")));
    assert_eq!(r.len(), 6, "{r:?}");
}

#[test]
fn never_reads_the_accounts() {
    let f = found();
    let all = format!("{f:?}");
    assert!(!all.contains("FAKE-TOKEN"));
    assert!(!all.contains("FAKE-CLIENT"));
}

#[test]
fn not_a_settings_file() {
    assert_eq!(parse("[]", regex, filter), Err(NotSettings));
    assert_eq!(parse("nope", regex, filter), Err(NotSettings));
    assert_eq!(parse(r#"{"nicknames": 5}"#, regex, filter), Err(NotSettings));
    // A settings file with none of these keys is just empty.
    assert_eq!(parse("{}", regex, filter), Ok(Extras::default()));
}

#[test]
fn nicknames_need_an_id() {
    let mut f = found();
    f.resolve_nicknames(&HashMap::from([("pajlada".to_owned(), "11148817".to_owned()), ("longname".to_owned(), "7".to_owned())]));
    assert_eq!(f.nicknames.len(), 2);
    assert!(reasons(&f).contains(&("ghost_user_404", "Nickname: not found on Twitch.")));
    let mut g = found();
    g.drop_nicknames("sign in to look them up.");
    assert!(g.nicknames.is_empty());
    assert!(reasons(&g).contains(&("pajlada", "Nickname: sign in to look them up.")));
}

#[test]
fn yours_stay_and_only_new_rows_count() {
    let mut f = found();
    f.resolve_nicknames(&HashMap::from([("pajlada".to_owned(), "1".to_owned()), ("longname".to_owned(), "2".to_owned())]));
    let mut h = Highlights::default();
    h.ignores.push(IgnoreRule { pattern: "SPOILER".into(), ..IgnoreRule::default() });
    h.never.push("nightbot".into());
    h.filters.push(FilterDef { id: "1b2c3d4e".into(), name: "mine".into(), expr: "author.mod".into() });
    let mut users = BTreeMap::new();
    users.insert("1".to_owned(), UserNote { nickname: "Mine".into(), ..UserNote::default() });
    users.insert("2".to_owned(), UserNote { note: "a note".into(), ..UserNote::default() });

    let p = plan(&f, &h, &users);
    assert_eq!((p.ignores, p.never, p.muted, p.filters, p.nicknames), (2, 1, 2, 2, 1));
    assert_eq!(p.already, 3);
    assert_eq!(p.added(), 8);
    // The plan works on copies.
    assert_eq!(h.ignores.len(), 1);

    let mut out = start(&f);
    apply_lists(&f, &mut h, &mut out);
    apply_nicknames(&f, &mut users, &mut out);
    assert_eq!(out, p);
    assert_eq!(users["1"].nickname, "Mine");
    assert_eq!(users["2"].nickname, "a nickname that is much l");
    assert_eq!(users["2"].note, "a note");
    // Chatterino's id was taken: the next free one.
    let ids: Vec<&str> = h.filters.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ["1b2c3d4e", "ch1", "9a8b7c6d"]);
    // A second run adds nothing.
    let again = plan(&f, &h, &users);
    assert_eq!(again.added(), 0);
}

#[test]
fn limits_cut_and_say_so() {
    let f = found();
    let h = Highlights {
        filters: (0..MAX_FILTERS)
            .map(|i| FilterDef { id: format!("f{i}"), name: format!("f{i}"), expr: format!("x{i}") })
            .collect(),
        ..Highlights::default()
    };
    let p = plan(&f, &h, &BTreeMap::new());
    assert_eq!(p.filters, 0);
    assert!(p.skipped.iter().any(|s| s.pattern == "Subs only" && s.reason == "Over the 50 limit."));
}
