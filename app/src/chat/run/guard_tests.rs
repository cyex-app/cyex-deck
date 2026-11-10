//! V110-F87 `guard`'s tests: an automation's line, run without the app.

use std::sync::Arc;

use cyex_chat::commands::chain::{split_chain, split_foreach};
use cyex_chat::commands::{self, Host, Items, Params, taint};

use super::super::super::typed::{self, INVISIBLE};
use super::super::reserved;
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

/// The user's own commands in these tests: a `/say` that repeats its words.
const CUSTOM: &[(&str, &str)] = &[("say", "$1-"), ("out", "/timeout $1 60")];

fn fill(body: &str, args: &str) -> Result<String, String> {
    let mut p = Params::new(args);
    p.set("nick", "chatter");
    p.taint = true;
    commands::run(&commands::parse(body).unwrap(), &p, &mut NoHost).ok_or("missing".into())
}

/// `line()` for an automation, without the app: the same guards in the same places.
fn line(text: &str, custom: bool, did: &mut Vec<Did>) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    if let Some(body) = text.strip_prefix("//") {
        own(body, REFILL)?;
        return line(&fill(&format!("/{body}"), "")?, false, did);
    }
    let known = |n: &str| reserved(n) || (custom && CUSTOM.iter().any(|c| c.0 == n));
    let Some((cmd, marked)) = typed::command(text, known) else {
        did.push(Did::Say(say(text)));
        return Ok(());
    };
    let cmd = cmd.to_ascii_lowercase();
    name(&cmd)?;
    let words = plain(marked);
    let args = args(&cmd, marked)?;
    if custom && let Some((_, body)) = CUSTOM.iter().find(|c| c.0 == cmd) {
        return line(&fill(body, &words)?, false, did);
    }
    match cmd.as_str() {
        "chain" => split_chain(&args).iter().try_for_each(|part| line(part, true, did)),
        "foreach" => {
            let (items, command) = split_foreach(&args).ok_or("usage")?;
            own(&command, LOOP)?;
            items.iter().try_for_each(|item| line(&fill(&command, &plain(item))?, true, did))
        }
        "runin" => line(args.split_once(char::is_whitespace).ok_or("usage")?.1, true, did),
        "timer" => {
            did.push(Did::Timer(args));
            Ok(())
        }
        _ => {
            did.push(Did::Run(cmd, args));
            Ok(())
        }
    }
}

/// What a rule answering `response` does when a chatter's words are `args`.
fn automation(response: &str, args: &str) -> Result<Vec<Did>, String> {
    let mut did = Vec::new();
    line(&fill(response, args)?, true, &mut did)?;
    // Nothing leaves with a mark on it.
    for d in &did {
        let (Did::Say(t) | Did::Run(_, t) | Did::Timer(t)) = d;
        assert!(!taint::has(t), "{d:?}");
    }
    Ok(did)
}

#[test]
fn the_chatters_words_are_said_never_run() {
    // "!say /ban victim" on a rule that answers "$1-".
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
