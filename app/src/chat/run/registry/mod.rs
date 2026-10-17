//! V1051-04 The registry of every command the app handles: prose written here, facts read
//! from the code (roles, anonymous use, plugin reservation), checked both ways by the tests
//! and written to `docs/commands/registry.json` by `tools/commands/registry.ps1`. `/help`
//! reads the same table.

mod app;
mod cosmetics;
mod emotes;
#[cfg(test)]
mod functions;
#[cfg(test)]
mod language;
#[cfg(test)]
mod render;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod variables;
mod twitch;

use serde::Serialize;

use super::builtin::OWN;

/// What a command sends or changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Changes {
    /// A line only you see.
    Local,
    /// A chat message.
    Chat,
    /// A Twitch action.
    Twitch,
    SevenTv,
    Settings,
    Window,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Arg {
    pub name: &'static str,
    pub required: bool,
    /// text|word|name|login|channel|number|duration|combo|paint|badge|command
    pub kind: &'static str,
    pub values: &'static [&'static str],
    pub note: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub input: &'static str,
    pub output: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cmd {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    /// The words in `/help`, in table order; `None` = not listed there.
    #[serde(skip)]
    pub help: Option<&'static str>,
    pub syntax: &'static str,
    pub args: &'static [Arg],
    pub summary: &'static str,
    pub changes: Changes,
    pub changes_note: &'static str,
    pub examples: &'static [Example],
    /// The ones this command's own code gives; the render adds Twitch's shared ones.
    #[serde(skip)]
    #[cfg_attr(not(test), allow(dead_code))]
    pub errors: &'static [&'static str],
    pub spec: &'static str,
    pub notes: &'static str,
}

pub(super) const fn ex(input: &'static str, output: &'static str) -> Example {
    Example { input, output }
}

pub(super) const fn req(name: &'static str, kind: &'static str) -> Arg {
    Arg {
        name,
        required: true,
        kind,
        values: &[],
        note: "",
    }
}

pub(super) const fn opt(name: &'static str, kind: &'static str) -> Arg {
    Arg {
        name,
        required: false,
        kind,
        values: &[],
        note: "",
    }
}

impl Arg {
    pub(super) const fn values(self, values: &'static [&'static str]) -> Self {
        Self { values, ..self }
    }

    pub(super) const fn note(self, note: &'static str) -> Self {
        Self { note, ..self }
    }
}

impl Cmd {
    /// An app command that only shows a line to you.
    pub(super) const fn new(
        name: &'static str,
        syntax: &'static str,
        summary: &'static str,
        spec: &'static str,
    ) -> Self {
        Self {
            name,
            aliases: &[],
            help: None,
            syntax,
            args: &[],
            summary,
            changes: Changes::Local,
            changes_note: "",
            examples: &[],
            errors: &[],
            spec,
            notes: "",
        }
    }

    pub(super) const fn alias(self, aliases: &'static [&'static str]) -> Self {
        Self { aliases, ..self }
    }

    pub(super) const fn help(self, help: &'static str) -> Self {
        Self {
            help: Some(help),
            ..self
        }
    }

    pub(super) const fn args(self, args: &'static [Arg]) -> Self {
        Self { args, ..self }
    }

    pub(super) const fn changes(self, changes: Changes, note: &'static str) -> Self {
        Self {
            changes,
            changes_note: note,
            ..self
        }
    }

    pub(super) const fn ex(self, examples: &'static [Example]) -> Self {
        Self { examples, ..self }
    }

    pub(super) const fn errs(self, errors: &'static [&'static str]) -> Self {
        Self { errors, ..self }
    }

    pub(super) const fn notes(self, notes: &'static str) -> Self {
        Self { notes, ..self }
    }
}

/// Every command, in the order `/help` lists them: the app's, then Twitch's.
pub(super) fn all() -> impl Iterator<Item = &'static Cmd> {
    app::BEFORE
        .iter()
        .chain(cosmetics::CMDS)
        .chain(emotes::CMDS)
        .chain(app::AFTER)
        .chain(twitch::CMDS)
}

/// Whether the app (not Twitch) handles it.
pub(super) fn is_app(name: &str) -> bool {
    OWN.contains(&name)
}

/// What `/help` says.
pub fn help_line() -> String {
    let words = |app: bool| -> String {
        all()
            .filter(|c| is_app(c.name) == app)
            .filter_map(|c| c.help)
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{}. Mods: {} and Twitch’s other mod commands. Your own commands are in Settings → Commands.",
        words(true),
        words(false)
    )
}
