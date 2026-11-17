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
    incoming("/x", func, false, &|_| true, &|_| false).map(|c| c.row["/x ".len()..].to_owned())
}

/// V111-56 The menu entry by name, and the one that runs the body (when the name stays cyex's).
fn menu_of(func: &str) -> (String, Option<String>) {
    let c = incoming("/x", func, true, &|_| true, &|_| false).unwrap();
    (c.menu.unwrap().unwrap(), c.menu_kept)
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
        // V111-56 One row for typed and the menu: empty (or the alt text) typed, filled from a menu.
        ("hi {user.name;you}{msg.text}{msg.id}{input.text}{element.copytext}", "hi $if(nick,$(nick),you)$(msg)$(msg-id)"),
        ("{user.id} {user.display_name;x} {channel.id}", "$(user-id) $if(display-nick,$(display-nick),x) $(chan-id)"),
        ("{user_x} {a1}", "{user_x} {a1}"),
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
    assert_eq!(menu_of("/user {user.name}"), (".x=/x".into(), Some(".x=/user $(nick)".into())));
    assert_eq!(
        menu_of("{user;anon}: {msg.text} ({msg-id})"),
        (".x=/x".into(), Some(".x=$if(nick,$(nick),anon): $(msg) ($(msg-id))".into()))
    );
    // Chatterino's menu passes the message as the words.
    assert_eq!(menu_of("say {1+}"), (".x=/x $(msg)".into(), Some(".x=say $(msg)".into())));
    assert_eq!(menu_of("/timeout {1} 1"), (".x=/x $(msg)".into(), None));
    assert_eq!(menu_of("{2+}"), (".x=/x $(msg)".into(), None));
    assert_eq!(menu_of("x {input.text}"), (".x=/x".into(), Some(".x=x".into())));
}

#[test]
fn every_body_keeps_to_one_line_and_the_limit() {
    assert_eq!(one("a\nb\r\n\r\nc").unwrap(), "a b c");
    let c = incoming("/x", "a\nb", false, &|_| true, &|_| false).unwrap();
    assert_eq!(c.notes, ["Line breaks became spaces."]);
    assert_eq!(one(&"x".repeat(3000)).unwrap_err(), "Over 2.000 characters.");
    assert_eq!(one("   ").unwrap_err(), "Sends nothing.");
    assert_eq!(one("{user.name}").unwrap(), "$(nick)");
    assert_eq!(one("room {channel.id}").unwrap(), "room $(chan-id)");
    assert_eq!(one("{input.text}").unwrap_err(), "Only uses {input.text}, which cyex doesn’t fill in.");
    assert_eq!(one("#js return 1").unwrap_err(), "Runs JavaScript, which cyex doesn’t run.");
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
            ("/lookup", "/lookup /user $(nick)"),
            ("/quote", "/quote \"$(msg)\" - $if(nick,$(nick),someone)"),
            ("/first", "/first /timeout $1 1"),
            ("/id", "/id room $(chan-id)"),
            ("/brace", "/brace {1}} costs \\$5 \\\\o/ \\{paint}"),
            ("!lurk", "/lurk /me is lurking"),
            ("/привет", "/привет Привет, $1! 🎉"),
            ("/ban", "/ban /ban $1 spam"),
            ("/stream", "/stream /streamlink $(chan)"),
        ]
    );
    let by = |n: &str| f.commands.iter().find(|c| c.name == n).unwrap();
    assert_eq!(by("/lookup").menu, Some(Ok(".lookup=/lookup".into())));
    assert_eq!(by("/lookup").menu_kept.as_deref(), Some(".lookup=/user $(nick)"));
    assert_eq!(by("/quote").menu, Some(Ok(".quote=/quote".into())));
    assert_eq!(by("/quote").menu_kept.as_deref(), Some(".quote=\"$(msg)\" - $if(nick,$(nick),someone)"));
    assert_eq!(by("/first").menu, Some(Ok(".first=/first $(msg)".into())));
    assert_eq!(by("/first").menu_kept, None);
    assert_eq!(by("/hi").menu, None);
    assert!(by("/stream").notes.is_empty());
    assert_eq!(by("/stream").does, "Can’t run here: cyex has no /streamlink.");
    assert_eq!(by("/lookup").does, "Opens their user card. From a menu it fills in {user.name}; typed, it’s empty.");
    assert_eq!(by("/first").does, "Times someone out. From the message menu, the message counts as what you type after it.");
    assert_eq!(by("/where").does, "Sends it to chat.");
    assert!(by("/so").notes.is_empty(), "/shoutout is known");
    assert_eq!(by("!lurk").key, "lurk");
    assert_eq!(by("/привет").key, "привет");

    let skipped: Vec<(&str, &str)> = f.skipped.iter().map(|s| (s.pattern.as_str(), s.reason.as_str())).collect();
    assert_eq!(
        skipped,
        [
            ("/Hi", "Same name as /hi here, where case doesn’t count."),
            ("/hi", "/hi is above it, so Chatterino never ran this one."),
            ("/two words", "Names with spaces don’t work here."),
            ("/empty", "Sends nothing."),
            ("/paste", "Only uses {input.text}, which cyex doesn’t fill in."),
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
    assert_eq!(item("/first").menu_note, None);
    assert_eq!(item("/first").menu_kept, None);
    assert_eq!((p.room, p.limit, p.menu_room), (496, 500, 500));
    assert_eq!(p.skipped.len(), 8);
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
    assert_eq!((r.added, r.replaced, r.kept, r.menu), (9, 1, 2, 0), "{r:?}");
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
    // /lookup is there already (by its label).
    assert_eq!((r.menu, r.menu_cut), (2, 0));
    let menu: Vec<&str> = c.message_menu.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        menu,
        [
            "Logs=/logs $1",
            SUBMENU,
            ".lookup=/user $(nick)",
            ".quote=/quote",
            ".first=/first $(msg)",
            "-"
        ]
    );
    // Without a submenu one is made at the end.
    let mut fresh = Commands::default();
    apply(f, &mut fresh, &reserved, &[], true);
    let menu: Vec<&str> = fresh.message_menu.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(menu, [SUBMENU, ".lookup=/lookup", ".quote=/quote", ".first=/first $(msg)"]);
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
    assert_eq!(r.cut, ["/quote", "/first", "/id", "/brace", "!lurk", "/привет", "/stream"]);
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

/// Name, row, what it does, the menu entry by name, the one when the name is kept.
type Seen<'a> = (&'a str, &'a str, &'a str, Option<&'a str>, Option<&'a str>);

/// V111-56 A real commands.json: each command as it comes in, what the dialog says it does,
/// and its menu entry (by name when imported, its body when the name stays cyex's).
#[test]
fn v111_56_a_real_file() {
    let ours = |n: &str| matches!(n, "pin" | "unpin" | "translate" | "tl" | "follow" | "unfollow" | "namehistory" | "copy" | "me");
    let path = format!("{}/tests/fixtures/chatterino/commands-user.json", env!("CARGO_MANIFEST_DIR"));
    let f = parse(&std::fs::read_to_string(path).unwrap(), &ours).unwrap();
    let got: Vec<Seen> = f
        .commands
        .iter()
        .map(|c| {
            let menu = c.menu.as_ref().and_then(|m| m.as_deref().ok());
            (c.name.as_str(), c.row.as_str(), c.does.as_str(), menu, c.menu_kept.as_deref())
        })
        .collect();
    let from_menu = "From the message menu, the message counts as what you type after it.";
    assert_eq!(
        got,
        [
            (
                "/pin",
                "/pin /pin $(msg-id)",
                "Pins a message. From a menu it fills in {msg.id}; typed, it’s empty.",
                Some(".pin=/pin"),
                Some(".pin=/pin $(msg-id)")
            ),
            (
                "/tl",
                "/tl /translate $1-",
                &*format!("Translates it; only you see that. {from_menu}"),
                Some(".tl=/tl $(msg)"),
                Some(".tl=/translate $(msg)")
            ),
            (
                "/repeat",
                "/repeat $1-",
                &*format!("Sends what you type after it. {from_menu}"),
                Some(".repeat=/repeat $(msg)"),
                Some(".repeat=$(msg)")
            ),
            (
                "/follow",
                "/follow /follow $(nick)",
                "Follows that channel. From a menu it fills in {user.name}; typed, it’s empty.",
                Some(".follow=/follow"),
                Some(".follow=/follow $(nick)")
            ),
            (
                "/namehistory",
                "/namehistory /namehistory $(nick)",
                "Lists their past names. From a menu it fills in {user.name}; typed, it’s empty.",
                Some(".namehistory=/namehistory"),
                Some(".namehistory=/namehistory $(nick)")
            ),
            (
                "/msgid",
                "/msgid /copy $(msg-id)",
                "Copies it to the clipboard. From a menu it fills in {msg.id}; typed, it’s empty.",
                Some(".msgid=/msgid"),
                Some(".msgid=/copy $(msg-id)")
            ),
        ]
    );
    let by = |n: &str| f.commands.iter().find(|c| c.name == n).unwrap();
    assert_eq!(by("/pin").notes, ["Runs /pin, cyex’s name for /pinmessage."]);
    assert_eq!(by("/repeat").notes, ["Sends the text itself; cyex has no /send."]);
    assert!(by("/msgid").notes.is_empty());
    let skipped: Vec<(&str, &str)> = f.skipped.iter().map(|s| (s.pattern.as_str(), s.reason.as_str())).collect();
    assert_eq!(skipped, [("/channels", "Runs JavaScript, which cyex doesn’t run.")]);

    // Kept as cyex's, /pin, /tl, /follow and /namehistory still bring their menu entries.
    let mut c = Commands::default();
    let r = apply(f.clone(), &mut c, &ours, &[], true);
    assert_eq!((r.added, r.kept, r.menu), (2, 4, 6), "{r:?}");
    let menu: Vec<&str> = c.message_menu.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        menu,
        [
            SUBMENU,
            ".pin=/pin $(msg-id)",
            ".tl=/translate $(msg)",
            ".repeat=/repeat $(msg)",
            ".follow=/follow $(nick)",
            ".namehistory=/namehistory $(nick)",
            ".msgid=/msgid"
        ]
    );
    // Taken, each entry runs the imported command.
    let all: Vec<u32> = (0..6).collect();
    let mut c = Commands::default();
    let r = apply(f, &mut c, &ours, &all, true);
    assert_eq!((r.added, r.kept, r.menu), (6, 0, 6), "{r:?}");
    assert!(c.message_menu.iter().any(|r| r.text == ".pin=/pin"));
}

/// V111-56 Aliases only swap a whole first word.
#[test]
fn v111_56_aliases() {
    let row = |f: &str| incoming("/x", f, false, &|_| true, &|_| false).unwrap();
    assert_eq!(row("/PinMessage {msg.id} 5m").row, "/x /pin $(msg-id) 5m");
    assert_eq!(row("/send hi {1}").row, "/x hi $1");
    assert_eq!(row("/sender hi").row, "/x /sender hi");
    assert_eq!(incoming("/x", "/send", false, &|_| true, &|_| false).unwrap_err(), "Sends nothing.");
    // Another command from the same file.
    let c = incoming("/x", "/other {1}", false, &|_| false, &|n| n == "other").unwrap();
    assert_eq!(c.does, "Runs your /other.");
}
