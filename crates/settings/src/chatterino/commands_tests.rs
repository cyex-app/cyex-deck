// V111-06 Chatterino's commands.json: its placeholders, our syntax, conflicts and the limit.

use super::*;

/// A stand-in for the app's own command names.
fn reserved(name: &str) -> bool {
    matches!(name, "me" | "ban" | "timeout" | "shoutout" | "user" | "so")
}

fn fixture() -> Found {
    let path = format!("{}/tests/fixtures/chatterino/commands.json", env!("CARGO_MANIFEST_DIR"));
    parse(&std::fs::read_to_string(path).unwrap(), &reserved).unwrap()
}

fn one(func: &str) -> Result<String, String> {
    incoming("/x", func, false, &|_| true).map(|c| c.row["/x ".len()..].to_owned())
}

fn menu_of(func: &str) -> Result<String, String> {
    incoming("/x", func, true, &|_| true).and_then(|c| c.menu.unwrap())
}

fn row(text: &str) -> CommandRow {
    CommandRow {
        enabled: true,
        text: text.into(),
    }
}

fn rows(c: &Commands) -> Vec<&str> {
    c.lines.iter().map(|r| r.text.as_str()).collect()
}

#[test]
fn the_mapping_table() {
    let cases = [
        ("/me waves at {1} and {2+}", "/me waves at $1 and $2-"),
        ("{1}-{2}", "$(1)-$2"),
        ("{1+}-x", "$1--x"),
        ("{10} {12+}", "$(10) $(12-)"),
        ("{channel.name} {channel}", "$(chan) $(chan)"),
        ("{channel;ignored}", "$(chan)"),
        ("{stream.game} {stream.title}", "$(streamgame) $(streamtitle)"),
        ("{stream.game;nothing, really)}", "$if(streamgame,$(streamgame),nothing\\, really\\))"),
        ("{my.name} {my.id}", "$(my-nick) $(my-user-id)"),
        // Typed, Chatterino has no message: its variables give their alt text.
        ("hi {user.name;you}{msg.text}{msg.id}{input.text}{element.copytext}", "hi you"),
        // Unknown names, `{0}` and huge numbers stay text.
        ("{nope} {nope;alt} {0} {99999999999}", "{nope} {nope;alt} {0} {99999999999}"),
        ("{Channel.Name}", "{Channel.Name}"),
        // `{{` is a brace; Chatterino also drops a leading "{" and the pairs before a placeholder.
        ("a {{b}} c", "a {b}} c"),
        ("a{{{1}", "a$1"),
        ("{ x", "x"),
        // Our own specials are escaped.
        ("costs $5 \\o/ {paint} {combo:Mine} {other}", "costs \\$5 \\\\o/ \\{paint} \\{combo:Mine} {other}"),
        ("{1}{2}", "$1$2"),
        ("{a;}}", "{a;}}"),
    ];
    for (func, want) in cases {
        assert_eq!(one(func).as_deref(), Ok(want), "{func}");
    }
}

#[test]
fn the_mapping_table_in_the_menu() {
    assert_eq!(menu_of("/user {user.name}").unwrap(), ".x=/user $(nick)");
    assert_eq!(menu_of("{user;anon}: {msg.text} ({msg-id})").unwrap(), ".x=$if(nick,$(nick),anon): $(msg) ($(msg-id))");
    assert_eq!(menu_of("say {1+}").unwrap(), ".x=say $(msg)");
    assert_eq!(menu_of("/timeout {1} 1").unwrap_err(), "Uses single words of the message.");
    assert_eq!(menu_of("{2+}").unwrap_err(), "Uses single words of the message.");
    assert_eq!(menu_of("x {input.text}").unwrap_err(), "Uses the text in the input box.");
    assert_eq!(menu_of("x {element.copytext}").unwrap_err(), "Uses the word you clicked.");
}

#[test]
fn every_body_keeps_to_one_line_and_the_limit() {
    assert_eq!(one("a\nb\r\n\r\nc").unwrap(), "a b c");
    let c = incoming("/x", "a\nb", false, &|_| true).unwrap();
    assert_eq!(c.notes, ["Line breaks became spaces."]);
    assert_eq!(one(&"x".repeat(3000)).unwrap_err(), "Over 2.000 characters.");
    assert_eq!(one("   ").unwrap_err(), "Sends nothing.");
    assert_eq!(one("{user.name}").unwrap_err(), "Sends nothing.");
    assert_eq!(one("room {channel.id}").unwrap_err(), "Uses {channel.id}, which has no match here.");
}

#[test]
fn names() {
    assert_eq!(name("/hi"), Ok(("/hi".into(), None)));
    assert_eq!(name(" hi "), Ok(("/hi".into(), Some("Typed as /hi here.".into()))));
    assert_eq!(name("!so"), Ok(("/so".into(), Some("Typed as /so here, not !so.".into()))));
    assert_eq!(name(".gg"), Ok(("/gg".into(), Some("Saved as /gg; .gg still works.".into()))));
    assert_eq!(name("/привет"), Ok(("/привет".into(), None)));
    for (bad, why) in [
        ("", "No name."),
        ("/", "No name."),
        ("!", "No name."),
        ("/a b", "Names with spaces don’t work here."),
        ("/a\tb", "Names with spaces don’t work here."),
        ("/hi#forsen", "A # in a name means one channel here."),
        ("//x", "Names can’t start with // here."),
    ] {
        assert_eq!(name(bad).unwrap_err(), why, "{bad:?}");
    }
    assert_eq!(name(&format!("/{}", "a".repeat(40))).unwrap_err(), "Name over 32 characters.");
}

#[test]
fn reads_the_fixture() {
    let f = fixture();
    let got: Vec<(&str, &str)> = f.commands.iter().map(|c| (c.name.as_str(), c.row.as_str())).collect();
    assert_eq!(
        got,
        [
            ("/hi", "/hi /me waves at $1 and $2-"),
            ("/so", "/so /shoutout $1"),
            ("/where", "/where We are in $(chan) playing $if(streamgame,$(streamgame),nothing)"),
            ("/lookup", "/lookup /user"),
            ("/quote", "/quote \"\" - someone"),
            ("/first", "/first /timeout $1 1"),
            ("/brace", "/brace {1}} costs \\$5 \\\\o/ \\{paint}"),
            ("!lurk", "/lurk /me is lurking"),
            ("/привет", "/привет Привет, $1! 🎉"),
            ("/ban", "/ban /ban $1 spam"),
            ("/stream", "/stream /streamlink $(chan)"),
        ]
    );
    let by = |n: &str| f.commands.iter().find(|c| c.name == n).unwrap();
    assert_eq!(by("/lookup").menu, Some(Ok(".lookup=/user $(nick)".into())));
    assert_eq!(by("/quote").menu, Some(Ok(".quote=\"$(msg)\" - $if(nick,$(nick),someone)".into())));
    assert_eq!(by("/first").menu, Some(Err("Uses single words of the message.".into())));
    assert_eq!(by("/hi").menu, None);
    assert_eq!(by("/stream").notes, ["Runs /streamlink, which isn’t a command here."]);
    assert!(by("/so").notes.is_empty(), "/shoutout is known");
    assert_eq!(by("!lurk").key, "lurk");
    assert_eq!(by("/привет").key, "привет");

    let skipped: Vec<(&str, &str)> = f.skipped.iter().map(|s| (s.pattern.as_str(), s.reason.as_str())).collect();
    assert_eq!(
        skipped,
        [
            ("/id", "Uses {channel.id}, which has no match here."),
            ("/Hi", "Same name as /hi here, where case doesn’t count."),
            ("/hi", "/hi is above it, so Chatterino never ran this one."),
            ("/two words", "Names with spaces don’t work here."),
            ("/empty", "Sends nothing."),
            ("/paste", "Sends nothing."),
            ("/me2", "Sends nothing."),
            ("Row 19", "Not a command."),
            ("Row 20", "Not a command."),
        ]
    );
}

#[test]
fn files_that_arent_commands() {
    for text in ["", "not json", "{}", "{\"commands\": 3}", "42", "{\"highlighting\": {}}"] {
        assert_eq!(parse(text, &reserved), Err(NotCommands), "{text:?}");
    }
    assert_eq!(parse("{\"commands\": []}", &reserved).unwrap(), Found::default());
    // A bare list reads too.
    assert_eq!(parse("[{\"name\":\"/a\",\"func\":\"b\"}]", &reserved).unwrap().commands.len(), 1);
}

#[test]
fn a_huge_file_reads_its_first_commands() {
    let list: Vec<String> = (0..MAX_READ + 25)
        .map(|i| format!("{{\"name\":\"/c{i}\",\"func\":\"{}\"}}", "word ".repeat(50)))
        .collect();
    let f = parse(&format!("{{\"commands\":[{}]}}", list.join(",")), &reserved).unwrap();
    assert_eq!(f.commands.len(), MAX_READ);
    assert_eq!(f.unread, 25);
}

#[test]
fn preview_marks_conflicts() {
    let f = fixture();
    let current = Commands {
        lines: vec![row("# mine"), row("/hi#forsen only there"), row("/HI my own hi"), row("/where here")],
        ..Commands::default()
    };
    let p = plan(&f, &current, &reserved);
    let item = |n: &str| p.items.iter().find(|i| i.name == n).unwrap();
    assert_eq!(item("/hi").conflict, Conflict::Yours);
    assert_eq!(item("/hi").yours.as_deref(), Some("/HI my own hi"));
    assert_eq!(item("/where").conflict, Conflict::Yours);
    assert_eq!(item("/ban").conflict, Conflict::BuiltIn);
    assert_eq!(item("/so").conflict, Conflict::BuiltIn);
    assert_eq!(item("/stream").conflict, Conflict::None);
    assert_eq!(item("/first").menu_note.as_deref(), Some("Uses single words of the message."));
    assert_eq!((p.room, p.limit, p.menu_room), (496, 500, 500));
    assert_eq!(p.skipped.len(), 9);
    // The preview changed nothing.
    assert_eq!(rows(&current).len(), 4);
}

#[test]
fn apply_keeps_or_takes_per_conflict() {
    let f = fixture();
    let mut c = Commands {
        lines: vec![row("/HI my own hi"), row("/where here")],
        ..Commands::default()
    };
    let ids = |names: &[&str]| -> Vec<u32> {
        f.commands
            .iter()
            .enumerate()
            .filter(|(_, x)| names.contains(&x.name.as_str()))
            .map(|(i, _)| i as u32)
            .collect()
    };
    let take = ids(&["/hi", "/ban"]);
    let r = apply(f.clone(), &mut c, &reserved, &take, false);
    assert_eq!((r.added, r.replaced, r.kept, r.menu), (8, 1, 2, 0), "{r:?}");
    assert!(r.cut.is_empty());
    let got = rows(&c);
    assert_eq!(got[0], "/hi /me waves at $1 and $2-", "replaced in place");
    assert_eq!(got[1], "/where here", "kept mine");
    assert_eq!(got[2], NOTE);
    assert!(got.contains(&"/ban /ban $1 spam"));
    assert!(!got.iter().any(|r| r.starts_with("/so ")), "the built-in kept its name");
    assert!(c.message_menu.is_empty());
}

#[test]
fn apply_adds_menu_entries_under_one_submenu() {
    let f = fixture();
    let mut c = Commands {
        message_menu: vec![row("Logs=/logs $1"), row(SUBMENU), row(".lookup=/user $(nick)"), row("-")],
        ..Commands::default()
    };
    let r = apply(f.clone(), &mut c, &reserved, &[], true);
    // /lookup is there already; /first doesn't map.
    assert_eq!((r.menu, r.menu_cut), (1, 0));
    let menu: Vec<&str> = c.message_menu.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        menu,
        [
            "Logs=/logs $1",
            SUBMENU,
            ".lookup=/user $(nick)",
            ".quote=\"$(msg)\" - $if(nick,$(nick),someone)",
            "-"
        ]
    );
    // Without a submenu one is made at the end.
    let mut fresh = Commands::default();
    apply(f, &mut fresh, &reserved, &[], true);
    let menu: Vec<&str> = fresh.message_menu.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(menu, [SUBMENU, ".lookup=/user $(nick)", ".quote=\"$(msg)\" - $if(nick,$(nick),someone)"]);
}

#[test]
fn the_limit_cuts_the_last_ones_and_says_which() {
    let f = fixture();
    let full: Vec<CommandRow> = (0..MAX_LINES - 3).map(|i| row(&format!("/mine{i} x"))).collect();
    let mut c = Commands {
        lines: full,
        ..Commands::default()
    };
    let r = apply(f, &mut c, &reserved, &[], false);
    assert_eq!(r.added, 3);
    assert_eq!(r.cut, ["/quote", "/first", "/brace", "!lurk", "/привет", "/stream"]);
    assert_eq!(c.lines.len(), MAX_LINES);
    // No room left over for the note: it gave way to the commands.
    assert!(!rows(&c).contains(&NOTE));
}

#[test]
fn apply_twice_adds_nothing_new() {
    let f = fixture();
    let mut c = Commands::default();
    apply(f.clone(), &mut c, &reserved, &[], true);
    let before = c.clone();
    let r = apply(f, &mut c, &reserved, &[], true);
    assert_eq!((r.added, r.menu), (0, 0));
    assert_eq!(c, before);
}
