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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Commands {
    /// `/name body`, `/name#channel body`, `_name body`, `# comment`.
    pub lines: Vec<String>,
    /// `/set var key value`, read with `$get(var,key)`.
    pub vars: BTreeMap<String, String>,
    /// S1 Right-click on a name: `Label=command`, `@Submenu` then `.Label=command`, `-`.
    pub user_menu: Vec<String>,
    /// S1 Right-click on a message.
    pub message_menu: Vec<String>,
    /// S1 Right-click on a chat's name in its header.
    pub channel_menu: Vec<String>,
    /// S1 Buttons on the usercard: `Label=command`.
    pub usercard_buttons: Vec<String>,
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

fn clip(lines: &mut Vec<String>) {
    lines.truncate(MAX_LINES);
    for l in lines.iter_mut() {
        if l.chars().count() > MAX_LINE_CHARS {
            *l = l.chars().take(MAX_LINE_CHARS).collect();
        }
    }
}

impl Commands {
    /// V105-12 Drops the automations left empty. True when something went.
    pub fn prune_blank(&mut self) -> bool {
        let before = self.automations.len();
        self.automations
            .retain(|a| !(a.name.trim().is_empty() && a.trigger.trim().is_empty() && a.response.trim().is_empty()));
        before != self.automations.len()
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

    #[test]
    fn sanitize_limits() {
        let mut c = Commands {
            lines: vec!["x".repeat(3000)],
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
        assert_eq!(c.lines[0].len(), MAX_LINE_CHARS);
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
