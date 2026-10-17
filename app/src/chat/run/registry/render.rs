//! The registry as JSON: the table’s prose plus the facts read from the code.

use cyex_moderation::command::Need;
use serde::Serialize;

use super::language::{
    COMMAND_LINE, ESCAPING, FUNCTIONS, Function, GUIDE, Guide, PRECEDENCE, VARIABLES, Variable,
};
use super::{Cmd, all, cosmetics, is_app};
use crate::chat::run::builtin::{need, reserved, usable};

const CHAT_VIEWS: &str = "tabs, tiles, pop-outs and #channel windows";

/// What every Twitch command can end in, before its own.
const TWITCH_ERRORS: [&str; 3] = [
    "Sign in with a moderator account to use that.",
    "Still loading this channel.",
    "This account can’t moderate. Sign in again in Settings → Accounts.",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Works {
    anonymous: bool,
    roles: &'static str,
    chat_views: &'static str,
    plugins_can_take_it: bool,
    your_own_can_shadow: bool,
}

#[derive(Serialize)]
struct Entry {
    #[serde(flatten)]
    cmd: &'static Cmd,
    source: &'static str,
    works: Works,
    errors: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Doc {
    version: u32,
    regenerate: &'static str,
    command_line: &'static [&'static str],
    precedence: &'static [&'static str],
    escaping: &'static [&'static str],
    custom_commands: Vec<String>,
    guide: &'static [Guide],
    commands: Vec<Entry>,
    variables: &'static [Variable],
    functions: &'static [Function],
}

pub(super) fn roles(name: &str) -> &'static str {
    match need(name) {
        None => "anyone",
        Some(Need::Mod) => "mod",
        Some(Need::Lead) => "lead",
        Some(Need::Broadcaster) => "broadcaster",
    }
}

fn entry(cmd: &'static Cmd, anonymous: &[&str]) -> Entry {
    let mut errors: Vec<&'static str> = Vec::new();
    if !is_app(cmd.name) {
        errors.extend(TWITCH_ERRORS);
        errors.extend(need(cmd.name).map(Need::refusal));
    }
    errors.extend(cmd.errors);
    if matches!(cmd.name, "combo" | "paint" | "badge") {
        errors.extend(cosmetics::SHARED);
    }
    Entry {
        cmd,
        source: if is_app(cmd.name) { "app" } else { "twitch" },
        works: Works {
            anonymous: anonymous.contains(&cmd.name),
            roles: roles(cmd.name),
            chat_views: CHAT_VIEWS,
            plugins_can_take_it: !reserved(cmd.name),
            your_own_can_shadow: true,
        },
        errors,
    }
}

/// V106-13 The old `customCommands` lines (the website reads them), one per guide entry.
pub(super) fn custom_commands() -> Vec<String> {
    GUIDE
        .iter()
        .map(|g| {
            [g.intro, g.variables, g.more]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// Pretty JSON, two spaces, `\n`, a trailing newline; the table’s order.
pub(super) fn render() -> String {
    let anonymous = usable(None, false);
    let doc = Doc {
        version: 1,
        regenerate: "tools/commands/registry.ps1",
        command_line: COMMAND_LINE,
        precedence: PRECEDENCE,
        escaping: ESCAPING,
        custom_commands: custom_commands(),
        guide: GUIDE,
        commands: all().map(|c| entry(c, &anonymous)).collect(),
        variables: VARIABLES,
        functions: FUNCTIONS,
    };
    let mut out = serde_json::to_string_pretty(&doc).expect("the registry serializes");
    out.push('\n');
    out
}
