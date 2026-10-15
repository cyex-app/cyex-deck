//! The command language around the commands: the command line, precedence, escaping,
//! custom commands, variables and functions. Only the registry file and its tests read it.

use serde::Serialize;

use super::Example;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Variable {
    pub name: &'static str,
    pub syntax: &'static str,
    /// everywhere|menus|usercard|automations
    pub scope: &'static [&'static str],
    pub value: &'static str,
    pub empty: &'static str,
    pub examples: &'static [Example],
    pub spec: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Function {
    pub name: &'static str,
    pub syntax: &'static str,
    pub summary: &'static str,
    pub example: Example,
}

pub(super) const COMMAND_LINE: &[&str] = &[
    "/ always starts a command.",
    "A dot starts one only before a name the app knows (F14); .. never does.",
    "// fills in variables in what follows, then runs it.",
    "The command name is not case-sensitive.",
    "A line that isn’t a command goes to Twitch, signed in only.",
    "Text that Twitch would read as a command gets an invisible character in front (F20).",
];

pub(super) const PRECEDENCE: &[&str] = &[
    "1. Your own command for this channel (/name#channel).",
    "2. Your own command for every channel.",
    "3. A plugin’s command. Plugins can’t take the app’s or Twitch’s names.",
    "4. The app’s and Twitch’s commands.",
    "Inside a custom command’s output your own commands don’t run again; built-ins and plugins do. Commands calling commands stop after 8 levels.",
];

pub(super) const ESCAPING: &[&str] = &[
    "\\x is x as is.",
    "$\"…\" is literal text; \"\" inside it is one \".",
    "{ only starts one of the 8 7TV variables; \\{ is a brace.",
    "/chain bodies double a | that comes from a value, and /foreach a >; $cs() and $fs() do it by hand.",
    "Combo and item names with spaces go in quotes in /combo (\"…\" or “…”); /paint and /badge take the rest of the line.",
    "/paint \"None\" means an item called None.",
];

pub(super) const CUSTOM_COMMANDS: &[&str] = &[
    "/name body, /name#channel body (that channel only), _name body (a replacement, used as $(_name)), # comment.",
    "Menus and user card buttons: Label=command, @Submenu followed by .Label=command lines, and - for a divider.",
    "A line of just /Name in a menu shows as Name and runs /name $1-.",
];

pub(super) use super::functions::FUNCTIONS;
pub(super) use super::variables::VARIABLES;
