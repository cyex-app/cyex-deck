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
    "A dot starts one only before a name the app knows; .. never does.",
    "// fills in variables in what follows, then runs it.",
    "The command name is not case-sensitive.",
    "A line that isn’t a command goes to Twitch, signed in only.",
    "Text that Twitch would read as a command gets an invisible character in front.",
];

pub(super) const PRECEDENCE: &[&str] = &[
    "1. Your own command for this channel (/name#channel).",
    "2. Your own command for every channel.",
    "Two of your own commands with the same name: the top row runs.",
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GuideExample {
    pub row: &'static [&'static str],
    pub typed: &'static str,
    pub result: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Guide {
    pub id: &'static str,
    pub title: &'static str,
    pub intro: &'static str,
    pub variables: &'static str,
    pub example: GuideExample,
    pub more: &'static str,
}

/// V106-13 How to write your own commands, in plain words: Settings → Commands and the docs read this.
pub(super) const GUIDE: &[Guide] = &[
    Guide {
        id: "commands",
        title: "Your commands",
        intro: "Make a command of your own: a name that starts with /, and what it sends.",
        variables: "Words with a $ are variables: the app fills them in when the command runs. $1 is the first word you type after the name, $1- is everything after it.",
        example: GuideExample { row: &["/hi", "Hello $1!"], typed: "/hi bob", result: "Hello bob!" },
        more: "Add #channel to the name, like /hi#forsen, to use it in one chat only. If two rows have the same name, the top one runs. A row that starts with # is a note.",
    },
    Guide {
        id: "menus",
        title: "Menus",
        intro: "Your own entries in the right-click menus. Each row is a label and the command it runs.",
        variables: "$(nick) is the name you clicked, $(msg) the message.",
        example: GuideExample { row: &["Shoutout", "/shoutout $(nick)"], typed: "Right-click bob › Shoutout", result: "/shoutout bob" },
        more: "A label that starts with @ makes a submenu, one that starts with . goes into the submenu above it, and - draws a line. A label like /Name runs your command /name.",
    },
    Guide {
        id: "usercard",
        title: "Usercard buttons",
        intro: "Buttons on a usercard. Each row is a label and the command it runs.",
        variables: "$(nick) is the person on the card.",
        example: GuideExample { row: &["Wave", "/me waves at $(nick)"], typed: "Wave on bob’s card", result: "/me waves at bob" },
        more: "",
    },
    Guide {
        id: "timers",
        title: "Timers",
        intro: "Commands waiting to run later.",
        variables: "",
        example: GuideExample { row: &[], typed: "/timer -r 10m /me stretch", result: "/me stretch, 10 minutes later, even after a restart" },
        more: "Without -r a timer ends when the app closes.",
    },
    Guide {
        id: "automations",
        title: "Automations",
        intro: "When someone says the trigger, the response runs as if you typed it.",
        variables: "$(nick) is who said it, $1- what came after the trigger.",
        example: GuideExample { row: &["!discord", "Join us: discord.gg/yours"], typed: "bob: !discord", result: "Join us: discord.gg/yours" },
        more: "",
    },
];

pub(super) use super::functions::FUNCTIONS;
pub(super) use super::variables::VARIABLES;
