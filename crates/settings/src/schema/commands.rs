//! Custom commands (1.8), menu entries and usercard buttons (S1), timers kept through
//! restarts (S3) and chat automations (S4). Commands use Chatty's syntax
//! (`docs/research/chat-clients.md`); lines are kept as typed so errors can be shown.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

const MAX_LINES: usize = 500;
const MAX_LINE_CHARS: usize = 2000;
const MAX_VARS: usize = 500;
const MAX_TIMERS: usize = 50;
const MAX_AUTOMATIONS: usize = 100;

/// V106-10 One row of a command or menu list: the line as typed, and whether it is used.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandRow {
    pub enabled: bool,
    pub text: String,
}

/// Old files and exports hold plain strings: each one becomes a row that's on.
impl<'de> Deserialize<'de> for CommandRow {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Text(String),
            Row {
                #[serde(default = "yes")]
                enabled: bool,
                #[serde(default)]
                text: String,
            },
        }
        fn yes() -> bool {
            true
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Text(text) => Self { enabled: true, text },
            Raw::Row { enabled, text } => Self { enabled, text },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Commands {
    /// `/name body`, `/name#channel body`, `_name body`, `# comment`.
    pub lines: Vec<CommandRow>,
    /// `/set var key value`, read with `$get(var,key)`.
    pub vars: BTreeMap<String, String>,
    /// S1 Right-click on a name: `Label=command`, `@Submenu` then `.Label=command`, `-`.
    pub user_menu: Vec<CommandRow>,
    /// S1 Right-click on a message.
    pub message_menu: Vec<CommandRow>,
    /// S1 Right-click on a chat's name in its header.
    pub channel_menu: Vec<CommandRow>,
    /// S1 Buttons on the usercard: `Label=command`.
    pub usercard_buttons: Vec<CommandRow>,
    /// S3 `/timer -r` timers.
    pub timers: Vec<SavedTimer>,
    /// S4
    pub automations: Vec<Automation>,
    pub automations_on: bool,
    /// S4 Most automatic messages per chat in 30 s.
    pub automation_limit: u8,
}

impl Default for Commands {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            vars: BTreeMap::new(),
            user_menu: Vec::new(),
            message_menu: Vec::new(),
            channel_menu: Vec::new(),
            usercard_buttons: Vec::new(),
            timers: Vec::new(),
            automations: Vec::new(),
            automations_on: false,
            automation_limit: 6,
        }
    }
}

/// S3
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct SavedTimer {
    pub id: String,
    /// Channel login it runs in.
    pub channel: String,
    /// The account that sends it; `None` reads anonymously.
    pub account: Option<String>,
    /// Unix ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    pub command: String,
    /// V1051-04 Set by an automation (S4): stays one after a restart.
    pub automation: bool,
}

/// S4 How the trigger is matched against a message.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TriggerMatch {
    /// The first word, e.g. `!discord`.
    #[default]
    Command,
    Exact,
    StartsWith,
    Contains,
    Regex,
}

/// S4 Who may set a rule off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Access {
    #[default]
    Everyone,
    Subscribers,
    Vips,
    Moderators,
    Broadcaster,
}

/// S4 When someone says `trigger`, run `response` as if you typed it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Automation {
    pub enabled: bool,
    pub name: String,
    /// Channel logins it works in.
    pub channels: Vec<String>,
    pub trigger: String,
    pub matching: TriggerMatch,
    /// A command body: `$1-` is what followed the trigger, `$(nick)` who said it,
    /// `$(count)` how often the rule ran.
    pub response: String,
    /// Answer as a reply to the message.
    pub reply: bool,
    pub who: Access,
    pub live_only: bool,
    /// Per chat.
    pub cooldown_secs: u32,
    /// Per person.
    pub user_cooldown_secs: u32,
    /// Times it ran, for `$(count)`.
    #[ts(type = "number")]
    pub count: u64,
}

impl Default for Automation {
    fn default() -> Self {
        Self {
            enabled: true,
            name: String::new(),
            channels: Vec::new(),
            trigger: String::new(),
            matching: TriggerMatch::Command,
            response: String::new(),
            reply: false,
            who: Access::Everyone,
            live_only: false,
            cooldown_secs: 10,
            user_cooldown_secs: 0,
            count: 0,
        }
    }
}

fn clip(rows: &mut Vec<CommandRow>) {
    rows.truncate(MAX_LINES);
    for r in rows.iter_mut() {
        if r.text.chars().count() > MAX_LINE_CHARS {
            r.text = r.text.chars().take(MAX_LINE_CHARS).collect();
        }
    }
}

/// Each row's text when on, else an empty line.
fn used(rows: &[CommandRow]) -> Vec<String> {
    rows.iter()
        .map(|r| if r.enabled { r.text.clone() } else { String::new() })
        .collect()
}

impl Commands {
    /// V106-10 The command lines the engine reads: an off row is an empty line, so errors
    /// keep their row numbers.
    pub fn command_lines(&self) -> Vec<String> {
        used(&self.lines)
    }

    /// V106-10 A menu's lines; an off `@Submenu` row also turns off the `.` rows under it.
    pub fn menu_lines(rows: &[CommandRow]) -> Vec<String> {
        let mut off_sub = false;
        rows.iter()
            .map(|r| {
                let t = r.text.trim();
                // Blank and note rows don't end a submenu.
                if t.is_empty() || t.starts_with('#') {
                } else if t.starts_with('@') {
                    off_sub = !r.enabled;
                } else if !t.starts_with('.') {
                    off_sub = false;
                }
                if !r.enabled || (t.starts_with('.') && off_sub) {
                    String::new()
                } else {
                    r.text.clone()
                }
            })
            .collect()
    }

    /// V105-12 Drops the automations and (V106-10) command rows left empty. True when
    /// something went.
    pub fn prune_blank(&mut self) -> bool {
        let before = self.automations.len();
        self.automations
            .retain(|a| !(a.name.trim().is_empty() && a.trigger.trim().is_empty() && a.response.trim().is_empty()));
        let mut gone = before != self.automations.len();
        for list in [
            &mut self.lines,
            &mut self.user_menu,
            &mut self.message_menu,
            &mut self.channel_menu,
            &mut self.usercard_buttons,
        ] {
            let n = list.len();
            list.retain(|r| !r.text.trim().is_empty());
            gone |= n != list.len();
        }
        gone
    }
}

impl Schema for Commands {
    fn sanitize(&mut self) {
        for list in [
            &mut self.lines,
            &mut self.user_menu,
            &mut self.message_menu,
            &mut self.channel_menu,
            &mut self.usercard_buttons,
        ] {
            clip(list);
        }
        while self.vars.len() > MAX_VARS {
            self.vars.pop_last();
        }
        self.timers.retain(|t| !t.id.is_empty() && !t.channel.is_empty() && !t.command.trim().is_empty());
        self.timers.truncate(MAX_TIMERS);
        self.automations.truncate(MAX_AUTOMATIONS);
        for a in &mut self.automations {
            for c in &mut a.channels {
                *c = c.trim().trim_start_matches('#').to_ascii_lowercase();
            }
            a.channels.retain(|c| !c.is_empty());
            a.channels.dedup();
            a.cooldown_secs = a.cooldown_secs.min(86_400);
            a.user_cooldown_secs = a.user_cooldown_secs.min(86_400);
        }
        self.automation_limit = self.automation_limit.clamp(1, 10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str) -> CommandRow {
        CommandRow { enabled: true, text: text.into() }
    }

    fn off(text: &str) -> CommandRow {
        CommandRow { enabled: false, text: text.into() }
    }

    #[test]
    fn plain_strings_read_as_rows_that_are_on() {
        let json = r#"{"lines":["/hi a",{"text":"/b x"},{"enabled":false,"text":"/c"}]}"#;
        let c: Commands = serde_json::from_str(json).unwrap();
        assert_eq!(c.lines, [row("/hi a"), row("/b x"), off("/c")]);
    }

    #[test]
    fn rows_round_trip() {
        let c = Commands { user_menu: vec![off("A=/a"), row("-")], ..Commands::default() };
        let back: Commands = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn command_lines_blank_off_rows() {
        let c = Commands { lines: vec![row("/a x"), off("/b y"), row("/c z")], ..Commands::default() };
        assert_eq!(c.command_lines(), ["/a x", "", "/c z"]);
    }

    #[test]
    fn menu_lines_turn_off_a_submenu() {
        let rows = [row("@Copy"), row(".A=/a"), off("@More"), row(".B=/b"), row(".C=/c"), row("Top=/t"), row(".D=/d")];
        assert_eq!(Commands::menu_lines(&rows), ["@Copy", ".A=/a", "", "", "", "Top=/t", ".D=/d"]);
        let rows = [row("@Copy"), off(".A=/a"), row(".B=/b")];
        assert_eq!(Commands::menu_lines(&rows), ["@Copy", "", ".B=/b"]);
        let rows = [off("@More"), row("# note"), row(""), row(".B=/b"), row("Top=/t"), row(".C=/c")];
        assert_eq!(Commands::menu_lines(&rows), ["", "# note", "", "", "Top=/t", ".C=/c"]);
    }

    #[test]
    fn prune_blank_drops_blank_rows() {
        let mut c = Commands {
            lines: vec![row("/a"), row("  "), off("")],
            usercard_buttons: vec![row("")],
            ..Commands::default()
        };
        assert!(c.prune_blank());
        assert_eq!(c.lines, [row("/a")]);
        assert!(c.usercard_buttons.is_empty());
        assert!(!c.prune_blank());
    }

    #[test]
    fn sanitize_limits() {
        let mut c = Commands {
            lines: std::iter::once(row(&"x".repeat(3000)))
                .chain(std::iter::repeat_with(|| row("y")).take(600))
                .collect(),
            automation_limit: 50,
            automations: vec![Automation {
                channels: vec![" #Forsen ".into(), "".into()],
                cooldown_secs: 1_000_000,
                ..Automation::default()
            }],
            timers: vec![SavedTimer::default()],
            ..Commands::default()
        };
        c.sanitize();
        assert_eq!(c.lines[0].text.len(), MAX_LINE_CHARS);
        assert_eq!(c.lines.len(), MAX_LINES);
        assert_eq!(c.automation_limit, 10);
        assert_eq!(c.automations[0].channels, ["forsen"]);
        assert_eq!(c.automations[0].cooldown_secs, 86_400);
        assert!(c.timers.is_empty());
    }

    #[test]
    fn prune_blank_drops_empty_automations() {
        let mut c = Commands {
            automations: vec![
                Automation::default(),
                Automation {
                    name: "hi".into(),
                    ..Automation::default()
                },
            ],
            ..Commands::default()
        };
        assert!(c.prune_blank());
        assert_eq!(c.automations.len(), 1);
        assert_eq!(c.automations[0].name, "hi");
        assert!(!c.prune_blank());
    }
}
