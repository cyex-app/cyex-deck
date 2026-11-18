//! V110-F87 `guard`'s tests: an automation's line, run without the app.

use std::sync::Arc;

use cyex_chat::commands::chain::{split_chain, split_foreach};
use cyex_chat::commands::{self, Host, Items, Params, taint};

use super::super::super::typed::{self, INVISIBLE};
use super::super::{MenuContext, Place, menu_values, reserved};
use super::*;

struct NoHost;

impl Host for NoHost {
    fn custom(&self, _: &str) -> Option<Arc<Items>> {
        None
    }
    fn get(&self, _: &str, _: Option<&str>) -> Option<String> {
        None
    }
    fn request(&mut self, _: &str, _: &[(String, String)]) -> Result<String, String> {
        Err("no".into())
    }
    fn random(&mut self, _: u64) -> u64 {
        0
    }
    fn now_ms(&self) -> i64 {
        0
    }
}

#[derive(Debug, PartialEq)]
enum Did {
    Say(String),
    Run(String, String),
    Timer(String),
    /// V110-F94 `/set`: its words, and whether the value is stored flagged.
    Set(String, bool),
}

fn said(text: &str) -> Did {
    Did::Say(text.into())
}

/// Text Twitch gets behind the invisible character.
fn hidden(text: &str) -> Did {
    Did::Say(format!("{INVISIBLE}{text}"))
}

fn ran(name: &str, args: &str) -> Did {
    Did::Run(name.into(), args.into())
}

/// The user's own commands in these tests: a `/say` that repeats its words. V111-56 And
/// Chatterino's imported ones, which read what the menu was opened on.
const CUSTOM: &[(&str, &str)] = &[
    ("say", "$1-"),
    ("out", "/timeout $1 60"),
    ("pinit", "/pin $(msg-id)"),
    ("msgid", "/copy $(msg-id)"),
    ("fol", "/follow $(nick)"),
    ("q", "/echo \"$(msg)\" - $if(nick,$(nick),someone)"),
];

thread_local! {
    /// V111-56 Like `Place::menu`: what the clicked entry was opened on.
    static MENU: std::cell::RefCell<Option<MenuContext>> = const { std::cell::RefCell::new(None) };
}

fn fill(body: &str, args: &str) -> Result<String, String> {
    let mut p = Params::new(args);
    p.set("nick", "chatter");
    fill_with(body, p, true)
}

fn fill_with(body: &str, mut p: Params, guarded: bool) -> Result<String, String> {
    p.taint = guarded;
    commands::run(&commands::parse(body).unwrap(), &p, &mut NoHost).ok_or("missing".into())
}

/// `line()` for an automation, without the app: the same guards in the same places.
fn line(text: &str, custom: bool, did: &mut Vec<Did>) -> Result<(), String> {
    line_at(text, custom, true, did)
}

/// V110-F94 `guarded`: `Place::guarded()`. Off is a line you typed, which marks on it turn on.
fn line_at(text: &str, custom: bool, guarded: bool, did: &mut Vec<Did>) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let guarded = guarded || taint::has(text);
    if let Some(body) = text.strip_prefix("//") {
        if guarded {
            own(body, REFILL)?;
        }
        let mut p = Params::new("");
        p.set("nick", "chatter");
        return line_at(&fill_with(&format!("/{body}"), p, guarded)?, false, guarded, did);
    }
    let known = |n: &str| reserved(n) || (custom && CUSTOM.iter().any(|c| c.0 == n));
    let Some((cmd, marked)) = typed::command(text, known) else {
        did.push(Did::Say(if guarded { say(text) } else { text.to_owned() }));
        return Ok(());
    };
    let cmd = cmd.to_ascii_lowercase();
    let (words, args) = if guarded {
        name(&cmd)?;
        (plain(marked), args(&cmd, marked)?)
    } else {
        (marked.to_owned(), marked.to_owned())
    };
    if custom && let Some((_, body)) = CUSTOM.iter().find(|c| c.0 == cmd) {
        // Like `params`: a menu's values reach the commands it runs.
        let mut p = Params::new(&words);
        MENU.with(|m| m.borrow().as_ref().map(|ctx| menu_values(&mut p, ctx)));
        return line_at(&fill_with(body, p, guarded)?, false, guarded, did);
    }
    match cmd.as_str() {
        "chain" => split_chain(&args).iter().try_for_each(|part| line_at(part, true, guarded, did)),
        "foreach" => {
            let (items, command) = split_foreach(&args).ok_or("usage")?;
            if guarded {
                own(&command, LOOP)?;
            }
            items
                .iter()
                .try_for_each(|item| line_at(&fill_with(&command, Params::new(&plain(item)), guarded)?, true, guarded, did))
        }
        "runin" => line_at(args.split_once(char::is_whitespace).ok_or("usage")?.1, true, guarded, did),
        // Like `timers::schedule`: the line it keeps runs as typed.
        "timer" => {
            did.push(Did::Timer(args));
            Ok(())
        }
        // Like `builtin::set_var`.
        "set" => {
            did.push(Did::Set(plain(&args), taint::has(&args)));
            Ok(())
        }
        // Like `builtin::run`'s "spam", after `spam::parse`.
        "spam" => {
            let text = args.split_once(' ').ok_or("usage")?.1.to_owned();
            did.push(Did::Say(if guarded { spam(after_first_word(&args)) } else { text }));
            Ok(())
        }
        _ => {
            did.push(Did::Run(cmd, args));
            Ok(())
        }
    }
}

/// Nothing leaves with a mark on it.
fn unmarked(did: Vec<Did>) -> Vec<Did> {
    for d in &did {
        let (Did::Say(t) | Did::Run(_, t) | Did::Timer(t) | Did::Set(t, _)) = d;
        assert!(!taint::has(t), "{d:?}");
    }
    did
}

/// What a rule answering `response` does when a chatter's words are `args`.
fn automation(response: &str, args: &str) -> Result<Vec<Did>, String> {
    let mut did = Vec::new();
    line(&fill(response, args)?, true, &mut did)?;
    Ok(unmarked(did))
}

/// V110-F94 A place as `menu_at` (`taint`) or the chat box makes it.
fn place(taint: bool) -> Place {
    Place {
        view: Some(1),
        key: (crate::chat::Identity::Anon, "chan".into()),
        automation: false,
        ui: true,
        plugins: Vec::new(),
        taint,
        menu: None,
    }
}

/// V110-F94 What a menu entry or usercard button `entry` does, clicked on evilname's message
/// `msg`: filled like `menu_at` fills it, run where `place` says.
fn clicked(entry: &str, msg: &str, place: &Place) -> Result<Vec<Did>, String> {
    let ctx = MenuContext {
        login: Some("evilname".into()),
        user_id: Some("7".into()),
        display: Some("/EvilName".into()),
        msg_id: Some("m1".into()),
        msg: Some(msg.into()),
        msg_time: None,
        input: None,
        copy: None,
    };
    let mut p = Params::new("evilname");
    menu_values(&mut p, &ctx);
    MENU.with(|m| *m.borrow_mut() = Some(ctx));
    let mut did = Vec::new();
    let guarded = place.guarded();
    let ran = fill_with(entry, p, guarded).and_then(|l| line_at(&l, true, guarded, &mut did));
    MENU.with(|m| *m.borrow_mut() = None);
    ran?;
    Ok(unmarked(did))
}

fn menu(entry: &str, msg: &str) -> Result<Vec<Did>, String> {
    clicked(entry, msg, &place(true))
}

#[test]
fn the_chatters_words_are_said_never_run() {
    // "!say /ban victim" on a rule that answers "$1-".
    assert_eq!(automation("$1-", "/me"), Ok(vec![hidden("/me")]));
    assert_eq!(automation("$1-", "/me hi"), Ok(vec![hidden("/me hi")]));
    assert_eq!(automation("$1-", "/ban victim"), Ok(vec![hidden("/ban victim")]));
    assert_eq!(automation("$1 $2", "/mod evil"), Ok(vec![hidden("/mod evil")]));
    assert_eq!(automation("$(nick): $1-", "/ban x"), Ok(vec![said("chatter: /ban x")]));
    assert_eq!(automation("$1-", "hello there"), Ok(vec![said("hello there")]));
    for typed in [
        ".ban x",
        "//ban x",
        "/me dances",
        ".me dances",
        "  /ban x",
        "\u{200B}/ban x",
        "\u{200B}\u{FEFF} \u{34F}.ban x",
        "\u{1}/ban x",
        "\u{E0000}/ban x",
        "/chain /ban a | /mod b",
        "/timer 5s /raid x",
    ] {
        assert_eq!(automation("$1-", typed), Ok(vec![hidden(typed)]), "{typed:?}");
    }
    // Spaces the rule wrote in front change nothing.
    assert_eq!(automation("  $1-", "/ban x"), Ok(vec![hidden("/ban x")]));
}

#[test]
fn the_rules_own_command_still_runs() {
    assert_eq!(automation("/timeout $(nick) 600", "x"), Ok(vec![ran("timeout", "chatter 600")]));
    assert_eq!(automation(".ban $1-", "evil | /mod evil"), Ok(vec![ran("ban", "evil | /mod evil")]));
    assert_eq!(automation("/me waves at $1-", "/ban x"), Ok(vec![ran("me", "waves at /ban x")]));
    // The rule picks the command, the message only which one.
    let pick = "$if(2,/timeout $1 $2,/ban $1)";
    assert_eq!(automation(pick, "evil 60"), Ok(vec![ran("timeout", "evil 60")]));
    assert_eq!(automation(pick, "evil"), Ok(vec![ran("ban", "evil")]));
    assert_eq!(automation("$ifeq(1,on,/slow 30,/slowoff)", "on"), Ok(vec![ran("slow", "30")]));
    assert_eq!(automation("$ifeq(1,on,/slow 30,/slowoff)", "/ban x"), Ok(vec![ran("slowoff", "")]));
    // A branch that is the message stays text.
    assert_eq!(automation("$if(1,$1-)", "/ban x"), Ok(vec![hidden("/ban x")]));
    assert_eq!(automation("$ifeq(1,/ban,$1 x,no)", "/ban"), Ok(vec![hidden("/ban x")]));
}

#[test]
fn the_message_never_names_the_command() {
    for (response, typed) in [
        ("/$1 x", "ban"),
        ("/ban$1", "x"),
        ("/ban$1-", " x"),
        ("/$1-", "/ban x"),
        ("/b$1 x", "an"),
        ("/$lower($1) x", "BAN"),
    ] {
        assert_eq!(automation(response, typed), Err(NAME.into()), "{response}");
    }
    // A dot before the message is a dot.
    assert_eq!(automation(".$1-", "ban x"), Ok(vec![hidden(".ban x")]));
    // `//` fills its body in again: "$request(…)" from a chatter must not get there.
    assert_eq!(automation("//echo $1-", "$(my-user-id)"), Err(REFILL.into()));
    assert_eq!(automation("//$1-", "ban x"), Err(REFILL.into()));
    assert_eq!(automation("//echo \\$(nick)", "x"), Ok(vec![ran("echo", "chatter")]));
}

#[test]
fn nothing_nested_runs_the_message() {
    // `/timer` keeps a line for later.
    assert_eq!(automation("/timer 5s $1-", "/raid x"), Err(TIMER.into()));
    assert_eq!(automation("/timer $1 /raid x", "5s"), Err(TIMER.into()));
    assert_eq!(automation("/timer 5s /unban $(nick)", ""), Err(TIMER.into()));
    assert_eq!(automation("/timer 5s /slowoff", "x"), Ok(vec![Did::Timer("5s /slowoff".into())]));
    assert_eq!(automation("/chain /timer 5s $1-", "/raid x"), Err(TIMER.into()));
    // The message can't add a part to a chain or start one.
    assert_eq!(
        automation("/chain /echo hi | $1-", "/ban x | /mod y"),
        Ok(vec![ran("echo", "hi"), hidden("/ban x | /mod y")])
    );
    assert_eq!(
        automation("/chain /echo $1- | /echo b", "a | /ban x"),
        Ok(vec![ran("echo", "a | /ban x"), ran("echo", "b")])
    );
    assert_eq!(automation("/runin other /chain hi $1-", "a | /ban x"), Ok(vec![said("hi a | /ban x")]));
    assert_eq!(automation("/runin other $1-", "/ban x"), Ok(vec![hidden("/ban x")]));
    // `/foreach`: the items may be the message's, the command never.
    assert_eq!(automation("/foreach a b > $1-", "/ban \\$1"), Err(LOOP.into()));
    assert_eq!(automation("/foreach x > \\$1 $1-", "y"), Err(LOOP.into()));
    assert_eq!(automation("/foreach a $1- /echo \\$1", "> /ban \\$1 >"), Err("usage".into()));
    assert_eq!(
        automation("/foreach $1- > /unban \\$1", "a > /mod"),
        Ok(vec![ran("unban", "a"), ran("unban", ">"), ran("unban", "/mod")])
    );
}

#[test]
fn the_users_own_commands_dont_run_it_either() {
    // "/say" repeats its words: fine typed by the user, text in an automation.
    assert_eq!(automation("/say $1-", "/ban x"), Ok(vec![hidden("/ban x")]));
    assert_eq!(automation("/say /ban $1", "x"), Ok(vec![hidden("/ban x")]));
    assert_eq!(automation("/out $1", "evil"), Ok(vec![ran("timeout", "evil 60")]));
    assert_eq!(
        automation("/chain /say $1- | /out $(nick)", "//mod x"),
        Ok(vec![hidden("//mod x"), ran("timeout", "chatter 60")])
    );
}

#[test]
fn a_plugins_answer_is_text() {
    for next in ["/ban x", " .mod y", "//echo x", "/me hi", "/nosuch"] {
        assert_eq!(plugin_answer(next, reserved), Err(PLUGIN.into()), "{next}");
    }
    assert_eq!(say(&plugin_answer("hello | there", reserved).unwrap()), "hello | there");
    assert_eq!(say(&plugin_answer(".gg", reserved).unwrap()), format!("{INVISIBLE}.gg"));
}

#[test]
fn plain_lines_are_not_touched() {
    for text in ["hello", "gg .ban", "1 > 0 | 2", "\u{200B}hi"] {
        assert_eq!(say(text), text);
    }
}

#[test]
fn v110_f94_a_menu_entry_says_the_clicked_message_never_runs_it() {
    // Without the guard (the place a typed line has) the click runs the chatter's ban.
    assert_eq!(clicked("$(msg)", "/ban x", &place(false)), Ok(vec![ran("ban", "x")]));
    assert_eq!(clicked("/say $(msg)", "/ban x", &place(false)), Ok(vec![ran("ban", "x")]));
    assert!(place(true).guarded() && !place(false).guarded());

    for msg in ["/ban x", ".mod evil", "//ban x", "/me hi", "\u{200B}/ban x", "/chain /ban a | /mod b", "/timer 5s /raid x"] {
        assert_eq!(menu("$(msg)", msg), Ok(vec![hidden(msg)]), "{msg:?}");
        assert_eq!(menu("/say $(msg)", msg), Ok(vec![hidden(msg)]), "{msg:?}");
        assert_eq!(menu("/chain /echo hi | $(msg)", msg), Ok(vec![ran("echo", "hi"), hidden(msg)]), "{msg:?}");
        assert_eq!(menu("/runin other $(msg)", msg), Ok(vec![hidden(msg)]), "{msg:?}");
    }
    assert_eq!(menu("$(msg)", "  /ban x"), Ok(vec![hidden("  /ban x")]));
    assert_eq!(menu("$(msg)", "hello there"), Ok(vec![said("hello there")]));
    // Their names are theirs too.
    assert_eq!(menu("$(display-nick) x", ""), Ok(vec![hidden("/EvilName x")]));
    assert_eq!(menu("$1 x", ""), Ok(vec![said("evilname x")]));
    for entry in ["/$(msg)", "/$(nick) x", "/b$(msg)", "/$lower($(msg))", "/$1"] {
        assert_eq!(menu(entry, "an x"), Err(NAME.into()), "{entry}");
    }
    assert_eq!(menu("//$(msg)", "ban x"), Err(REFILL.into()));
    assert_eq!(menu("//echo $(msg)", "$(my-user-id)"), Err(REFILL.into()));
    assert_eq!(menu("/foreach a b > $(msg)", "/ban \\$1"), Err(LOOP.into()));
    assert_eq!(menu("/timer 5s $(msg)", "/raid x"), Err(TIMER.into()));
    assert_eq!(menu("/timer 5m /unban $(nick)", ""), Err(TIMER.into()));
}

#[test]
fn v110_f94_a_menu_entrys_own_command_still_runs() {
    // The documented uses: the entry names the command, the click fills in plain words.
    assert_eq!(menu("/timeout $(nick) 600 $(msg)", "/ban x | /mod y"), Ok(vec![ran("timeout", "evilname 600 /ban x | /mod y")]));
    assert_eq!(menu("/ban $(nick)", ""), Ok(vec![ran("ban", "evilname")]));
    assert_eq!(menu("/shoutout $(nick)", ""), Ok(vec![ran("shoutout", "evilname")]));
    assert_eq!(menu("/echo $(msg)", "/ban x"), Ok(vec![ran("echo", "/ban x")]));
    assert_eq!(menu("/me waves at $(nick)", ""), Ok(vec![ran("me", "waves at evilname")]));
    // A command of yours behind the entry, and one picked by the click.
    assert_eq!(menu("/out $(nick)", ""), Ok(vec![ran("timeout", "evilname 60")]));
    assert_eq!(menu("$if(msg,/delete $(msg-id),/ban $(nick))", "spam"), Ok(vec![ran("delete", "m1")]));
    assert_eq!(menu("/chain /timeout $(nick) 60 | /echo done", ""), Ok(vec![ran("timeout", "evilname 60"), ran("echo", "done")]));
    assert_eq!(menu("/timer 5m /slowoff", ""), Ok(vec![Did::Timer("5m /slowoff".into())]));
}

#[test]
fn v110_f94_spam_and_set_keep_the_message_as_text() {
    assert_eq!(menu("/spam 3 $(msg)", ".ban x"), Ok(vec![hidden(".ban x")]));
    assert_eq!(automation("/spam 3 $1-", ".mod evil"), Ok(vec![hidden(".mod evil")]));
    assert_eq!(menu("/spam 3 /me $(msg)", ".ban x"), Ok(vec![said("/me .ban x")]));
    assert_eq!(menu("/spam 3 hi $(nick)", ""), Ok(vec![said("hi evilname")]));
    // V110-F99 The message's own "/me" is text; only the entry's /me makes an action.
    assert_eq!(automation("/spam 3 $1-", "/me dances"), Ok(vec![hidden("/me dances")]));
    assert_eq!(menu("/spam 3 $(msg)", "/me dances"), Ok(vec![hidden("/me dances")]));
    // What `/set var` takes from the message is stored flagged; the entry's own value isn't.
    assert_eq!(automation("/set var q $1-", "/ban x"), Ok(vec![Did::Set("var q /ban x".into(), true)]));
    assert_eq!(menu("/set var last $(nick)", ""), Ok(vec![Did::Set("var last evilname".into(), true)]));
    assert_eq!(automation("/set var mode on", "x"), Ok(vec![Did::Set("var mode on".into(), false)]));
}

#[test]
fn v110_f94_a_typed_line_with_a_stored_value_is_guarded() {
    // `commands::run` marks what `$get(var, …)` read when someone else's message stored it
    // (tested there); a typed line or a command of yours that comes out with marks on it is
    // guarded from there down.
    let typed = |text: &str| {
        let mut did = Vec::new();
        line_at(text, true, false, &mut did).map(|()| unmarked(did))
    };
    let q = taint::mark("/ban x");
    assert_eq!(typed(&q), Ok(vec![hidden("/ban x")]));
    assert_eq!(typed(&format!("/{}", taint::mark("ban x"))), Err(NAME.into()));
    assert_eq!(typed(&format!("/chain /echo a | {q}")), Ok(vec![ran("echo", "a"), hidden("/ban x")]));
    assert_eq!(typed(&format!("/timer 5s {q}")), Err(TIMER.into()));
    assert_eq!(typed(&format!("/timeout {} 60", taint::mark("evil"))), Ok(vec![ran("timeout", "evil 60")]));
    assert_eq!(typed(&format!("/set var b {q}")), Ok(vec![Did::Set("var b /ban x".into(), true)]));
    // What you type yourself runs as before.
    assert_eq!(typed("/ban x"), Ok(vec![ran("ban", "x")]));
    assert_eq!(typed("/say /ban x"), Ok(vec![ran("ban", "x")]));
    assert_eq!(typed("/timer 5s /raid x"), Ok(vec![Did::Timer("5s /raid x".into())]));
    assert_eq!(typed("/set var mode on"), Ok(vec![Did::Set("var mode on".into(), false)]));
}

/// V110-F99 What a built-in sends on someone's behalf (`/tl`'s translation) is text in a
/// guarded line; a line you typed goes out as it is.
#[test]
fn v110_f99_a_translation_of_the_message_is_text() {
    assert_eq!(sent(true, "/ban x".into()), format!("{INVISIBLE}/ban x"));
    assert_eq!(sent(true, " .mod y".into()), format!("{INVISIBLE} .mod y"));
    assert_eq!(sent(true, "hallo".into()), "hallo");
    assert_eq!(sent(false, "/me typed".into()), "/me typed");
    assert_eq!(after_first_word("3  /me hi"), "/me hi");
    assert_eq!(after_first_word("3"), "");
}

/// V111-56 A command of yours run from a menu reads what the menu was opened on, like
/// Chatterino's `{msg.id}`; typed, the same values are empty.
#[test]
fn v111_56_menu_values_reach_your_commands() {
    let typed = |text: &str| {
        let mut did = Vec::new();
        line_at(text, true, false, &mut did).map(|()| unmarked(did))
    };
    assert_eq!(menu("/pinit", "hi"), Ok(vec![ran("pin", "m1")]));
    assert_eq!(menu("/msgid", "hi"), Ok(vec![ran("copy", "m1")]));
    assert_eq!(menu("/fol", "hi"), Ok(vec![ran("follow", "evilname")]));
    assert_eq!(menu("/q", "/ban x"), Ok(vec![ran("echo", "\"/ban x\" - evilname")]));
    assert_eq!(typed("/pinit"), Ok(vec![ran("pin", "")]));
    assert_eq!(typed("/fol"), Ok(vec![ran("follow", "")]));
    assert_eq!(typed("/q"), Ok(vec![ran("echo", "\"\" - someone")]));
    // Chatterino's menu passes the message as the words: still only said, never run.
    assert_eq!(menu("/say $(msg)", "/ban x"), Ok(vec![hidden("/ban x")]));
}

/// V111-59 A room for `$(last-msg)`: bob's newest line here is "/ban x" (id b2).
#[derive(Default)]
struct Room59 {
    stop: Option<String>,
    foreign: std::cell::Cell<bool>,
}

impl Host for Room59 {
    fn custom(&self, _: &str) -> Option<Arc<Items>> {
        None
    }
    fn get(&self, _: &str, _: Option<&str>) -> Option<String> {
        None
    }
    fn request(&mut self, _: &str, _: &[(String, String)]) -> Result<String, String> {
        Err("no".into())
    }
    fn lookup(&mut self, name: &str, p: &Params) -> Option<Option<String>> {
        let found = super::super::last_value(name, p, |login| (login == "bob").then(|| ("b2".into(), "/ban x".into())))?;
        Some(match found {
            Ok(v) => Some(v),
            Err(why) => {
                self.stop.get_or_insert(why);
                None
            }
        })
    }
    fn foreign(&self) -> bool {
        self.foreign.get()
    }
    fn set_foreign(&mut self, on: bool) {
        self.foreign.set(on);
    }
    fn random(&mut self, _: u64) -> u64 {
        0
    }
    fn now_ms(&self) -> i64 {
        0
    }
}

#[derive(Clone, Copy, Debug)]
enum At {
    /// `/t @bob, more` typed in the chat box.
    Typed,
    /// The message menu on bob's "hello world", with "draft" in the chat box and "world"
    /// right-clicked.
    Menu,
    /// A usercard button on bob's card.
    Card,
}

/// Chatterino's `func` as the import makes it: the row's body and the menu entry's command.
fn imported(func: &str) -> (String, String) {
    let json = serde_json::json!({ "commands": [{ "name": "/t", "func": func, "showInMsgContextMenu": true }] });
    let found = cyex_settings::chatterino::parse_commands(&json.to_string(), &reserved).unwrap();
    let c = &found.commands[0];
    let body = c.row.strip_prefix("/t ").unwrap().to_owned();
    let entry = c.menu.clone().unwrap().unwrap();
    (body, entry.split_once('=').unwrap().1.to_owned())
}

/// What the app's values are in these tests (`values`), with `ctx` like `params` adds it.
fn params59(args: &str, ctx: Option<&MenuContext>) -> Params {
    let live = cyex_twitch::gql::ChannelInfo {
        id: "99".into(),
        login: "chan".into(),
        name: "Chan".into(),
        avatar: None,
        offline_image: None,
        stream: Some(cyex_twitch::gql::StreamInfo {
            rerun: false,
            title: "Speedrun".into(),
            game: Some("Celeste".into()),
            viewers: 1,
            started_at: String::new(),
        }),
    };
    let me = cyex_settings::schema::Account {
        user_id: "7".into(),
        login: "me".into(),
        display_name: "Me".into(),
        ..Default::default()
    };
    let mut p = super::super::values("chan", Some("99"), &["chan".into()], Some(&live), Some(&me));
    if let Some(ctx) = ctx {
        menu_values(&mut p, ctx);
    }
    p.set_args(args);
    p
}

/// Runs `body` with `p`: the line it makes, still marked, or why it stopped.
fn run59(body: &str, mut p: Params, guarded: bool) -> Result<String, String> {
    p.taint = guarded;
    let mut h = Room59::default();
    let out = commands::run(&commands::parse(body).unwrap(), &p, &mut h);
    match h.stop {
        Some(why) => Err(why),
        None => out.ok_or_else(|| "missing".into()),
    }
}

/// V111-59 Chatterino's `func`, imported, then run `at` the way the app runs it: the line it
/// makes (still marked) and whether that line is guarded.
fn chatterino(func: &str, at: At) -> Result<(String, bool), String> {
    let (body, entry) = imported(func);
    let card = MenuContext {
        login: Some("bob".into()),
        user_id: Some("5".into()),
        display: Some("Bob".into()),
        ..Default::default()
    };
    match at {
        At::Typed => run59(&body, params59("@bob, more", None), false).map(|l| (l, false)),
        At::Menu => {
            let ctx = MenuContext {
                msg_id: Some("m1".into()),
                msg: Some("hello world".into()),
                msg_time: Some(1),
                input: Some("draft".into()),
                copy: Some("world".into()),
                ..card
            };
            // `menu_at`: the entry is filled, then runs /t with the message as its words.
            let line = run59(&entry, params59("bob", Some(&ctx)), true)?;
            let words = line.strip_prefix("/t").ok_or("not /t")?.trim_start();
            run59(&body, params59(&plain(words), Some(&ctx)), true).map(|l| (l, true))
        }
        At::Card => run59(&body, params59("bob", Some(&card)), true).map(|l| (l, true)),
    }
}

/// V111-59 Every placeholder Chatterino has (docs/research/chatterino-placeholders.md): what
/// the import makes of it, and its value typed, from the message menu and from a usercard
/// button, as Chatterino gives it. `-` is the alt text, where Chatterino has no value.
#[test]
fn v111_59_every_chatterino_placeholder_has_its_value() {
    let cases: &[(&str, &str, [&str; 3])] = &[
        ("{1}", "$1", ["@bob,", "hello", "bob"]),
        ("{1+}", "$1-", ["@bob, more", "hello world", "bob"]),
        ("{2;-}", "$if(2,$2,-)", ["more", "world", "-"]),
        ("{channel.name}", "$(chan)", ["chan"; 3]),
        ("{channel}", "$(chan)", ["chan"; 3]),
        ("{channel.id;-}", "$if(chan-id,$(chan-id),-)", ["99"; 3]),
        ("{stream.game;-}", "$if(streamgame,$(streamgame),-)", ["Celeste"; 3]),
        ("{stream.title;-}", "$if(streamtitle,$(streamtitle),-)", ["Speedrun"; 3]),
        ("{my.name;-}", "$if(my-nick,$(my-nick),-)", ["me"; 3]),
        ("{my.id;-}", "$if(my-user-id,$(my-user-id),-)", ["7"; 3]),
        ("{user.name;-}", "$if(nick,$(nick),-)", ["-", "bob", "bob"]),
        ("{user;-}", "$if(nick,$(nick),-)", ["-", "bob", "bob"]),
        ("{user.id;-}", "$if(user-id,$(user-id),-)", ["-", "5", "5"]),
        // Chatterino's usercard leaves this one empty; here the card knows the name.
        ("{user.display_name;-}", "$if(display-nick,$(display-nick),-)", ["-", "Bob", "Bob"]),
        ("{msg.id;-}", "$if(msg-id,$(msg-id),-)", ["-", "m1", "-"]),
        ("{msg-id;-}", "$if(msg-id,$(msg-id),-)", ["-", "m1", "-"]),
        ("{msg.text;-}", "$if(msg,$(msg),-)", ["-", "hello world", "-"]),
        ("{message;-}", "$if(msg,$(msg),-)", ["-", "hello world", "-"]),
        ("{input.text;-}", "$if(input-text,$(input-text),-)", ["-", "draft", "-"]),
        ("{element.copytext;-}", "$if(copy-text,$(copy-text),-)", ["-", "world", "-"]),
        ("{user.last_message.id}", "$(last-msg-id)", ["b2"; 3]),
        ("{user.last_message.text}", "$(last-msg)", ["/ban x"; 3]),
        // Escapes and names it doesn't know.
        ("a{{{1}", "a$1", ["a@bob,", "ahello", "abob"]),
        ("{nope;x} {{b}", "{nope;x} {b}", ["{nope;x} {b}"; 3]),
    ];
    for (func, ours, want) in cases {
        assert_eq!(imported(func).0, *ours, "{func}: import");
        for (at, want) in [At::Typed, At::Menu, At::Card].into_iter().zip(want) {
            let (line, _) = chatterino(func, at).unwrap_or_else(|e| panic!("{func} {at:?}: {e}"));
            assert_eq!(taint::clean(&line), *want, "{func} {at:?}");
        }
    }
    // {time}: the local time, everywhere.
    assert_eq!(imported("{time}").0, "$(time)");
    for at in [At::Typed, At::Menu, At::Card] {
        let (line, _) = chatterino("{time}", at).unwrap();
        let t = taint::clean(&line);
        let ok = t.len() == 5 && t.as_bytes()[2] == b':' && t.chars().filter(char::is_ascii_digit).count() == 4;
        assert!(ok, "{at:?}: {t}");
    }
}

/// V111-59 `{user.last_message.*}` reads whose the menu or card is, else the first word, and
/// stops the command like Chatterino when there's no one or nothing.
#[test]
fn v111_59_the_last_message_needs_someone_who_said_something() {
    let typed = |args: &str| run59("/echo $(last-msg)", params59(args, None), false);
    assert_eq!(typed("bob").map(|l| taint::clean(&l)), Ok("/echo /ban x".into()));
    assert_eq!(typed("@bob,").map(|l| taint::clean(&l)), Ok("/echo /ban x".into()));
    assert_eq!(typed(""), Err("This command needs a username.".into()));
    assert_eq!(typed("nobody"), Err("No recent message from nobody is in this chat.".into()));
    // An alt text doesn't help: there's no message to read (Chatterino stops too).
    assert_eq!(
        run59("$if(last-msg-id,$(last-msg-id),none)", params59("nobody", None), false),
        Err("No recent message from nobody is in this chat.".into())
    );
}

/// V111-59 The new values come from someone's message or a click: what they hold is said,
/// never run, wherever the command runs.
#[test]
fn v111_59_new_values_never_become_a_command() {
    let did = |line: &str, guarded: bool| {
        let mut did = Vec::new();
        line_at(line, true, guarded, &mut did).map(|()| unmarked(did))
    };
    // Typed: bob's newest message is "/ban x".
    for func in ["{user.last_message.text}", "{user.last_message.text;-}", "/say {user.last_message.text}"] {
        let (line, guarded) = chatterino(func, At::Typed).unwrap();
        assert!(taint::has(&line), "{func}");
        assert_eq!(did(&line, guarded), Ok(vec![hidden("/ban x")]), "{func}");
    }
    // The command's own text still runs, with their message as plain words.
    let (line, guarded) = chatterino("/echo {user.last_message.text}", At::Typed).unwrap();
    assert_eq!(did(&line, guarded), Ok(vec![ran("echo", "/ban x")]));
    // From a menu: the chat box's text, the clicked word and their message.
    let ctx = MenuContext {
        login: Some("bob".into()),
        msg: Some("hi".into()),
        input: Some("/ban x".into()),
        copy: Some(".mod y".into()),
        ..Default::default()
    };
    for (entry, said) in [("$(input-text)", "/ban x"), ("$(copy-text)", ".mod y"), ("$(last-msg)", "/ban x")] {
        let line = run59(entry, params59("bob", Some(&ctx)), true).unwrap();
        assert_eq!(did(&line, true), Ok(vec![hidden(said)]), "{entry}");
        for bad in [format!("/{entry}"), format!("/chain /echo a | {entry}")] {
            let line = run59(&bad, params59("bob", Some(&ctx)), true).unwrap();
            assert_ne!(did(&line, true), Ok(vec![ran("ban", "x")]), "{bad}");
        }
    }
}
