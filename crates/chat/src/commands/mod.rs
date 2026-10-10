//! 1.8 Custom commands in Chatty's syntax (`docs/research/chat-clients.md`): one per
//! line as `/name body`, `/name#channel body` (that channel only), `_name body` (a
//! replacement used as `$(_name)`) or `# comment`. Bodies use `$1`, `$1-`, `$$1`,
//! `$(chan)` and functions like `$if`, `$replace`, `$json` and `$request`.

mod calc;
pub mod chain;
mod datetime;
mod eval;
mod json;
pub mod menu;
mod parse;
mod text;

use std::collections::HashMap;
use std::sync::Arc;

pub use eval::{Host, Params};
pub use parse::{Item, Items, ParseError};

/// Reads one body, e.g. an inline command from a menu entry.
pub fn parse(body: &str) -> Result<Items, ParseError> {
    parse::parse(body)
}

/// Fills in a body. `None` when a required value (`$$1`) was missing. Bodies that start
/// with `/chain` or `/foreach` escape what replacements put in, so it can't split them.
pub fn run(body: &Items, params: &Params, host: &mut dyn Host) -> Option<String> {
    let mut p = params.clone();
    if let Some(Item::Text(first)) = body.first() {
        let first = first.trim_start().to_ascii_lowercase();
        if first.starts_with("/chain ") {
            p.escape = Some('|');
        } else if first.starts_with("/foreach ") {
            p.escape = Some('>');
        }
    }
    eval::eval_top(body, &p, host)
}

/// One line of the list that didn't read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineError {
    /// From 0.
    pub line: usize,
    pub message: String,
}

/// The user's commands and replacements, ready to run.
#[derive(Default)]
pub struct CommandList {
    /// (name without `/`, channel or "") → body.
    commands: HashMap<(String, String), Arc<Items>>,
    replacements: HashMap<(String, String), Arc<Items>>,
    pub errors: Vec<LineError>,
}

impl CommandList {
    pub fn new(lines: &[String]) -> Self {
        let mut list = Self::default();
        for (i, raw) in lines.iter().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (head, body) = line.split_once(' ').unwrap_or((line, ""));
            let (name, channel) = head.split_once('#').unwrap_or((head, ""));
            let key = (
                name.to_ascii_lowercase(),
                channel.trim_start_matches('#').to_ascii_lowercase(),
            );
            let parsed = match parse::parse(body.trim_start()) {
                Ok(items) => Arc::new(items),
                Err(e) => {
                    list.errors.push(LineError {
                        line: i,
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            if let Some(n) = key.0.strip_prefix('/').filter(|n| !n.is_empty()) {
                list.commands.insert((n.to_owned(), key.1), parsed);
            } else if key.0.len() > 1 && key.0.starts_with('_') {
                list.replacements.insert(key, parsed);
            } else {
                list.errors.push(LineError {
                    line: i,
                    message: "Start with /name or _name".into(),
                });
            }
        }
        list
    }

    /// `/name` in `channel`: the channel's own version first.
    pub fn command(&self, name: &str, channel: &str) -> Option<Arc<Items>> {
        Self::find(&self.commands, name, channel)
    }

    /// `$(_name)` in `channel`.
    pub fn replacement(&self, name: &str, channel: &str) -> Option<Arc<Items>> {
        Self::find(&self.replacements, name, channel)
    }

    /// Command names, for completion.
    pub fn names(&self, channel: &str) -> Vec<String> {
        let mut names: Vec<String> = self
            .commands
            .keys()
            .filter(|(_, c)| c.is_empty() || c == channel)
            .map(|(n, _)| n.clone())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    fn find(map: &HashMap<(String, String), Arc<Items>>, name: &str, channel: &str) -> Option<Arc<Items>> {
        let name = name.to_ascii_lowercase();
        map.get(&(name.clone(), channel.to_ascii_lowercase()))
            .or_else(|| map.get(&(name, String::new())))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::eval::tests::TestHost;
    use super::*;

    #[test]
    fn list_reads_lines() {
        let lines: Vec<String> = [
            "# hello",
            "/hello Hello World!",
            "/hello#joshimuz Hello Joshimuz Chat!",
            "_m $ifeq(1,$(chan),,$$1: )",
            "/bad $nope(",
            "oops x",
        ]
        .map(String::from)
        .to_vec();
        let list = CommandList::new(&lines);
        assert_eq!(list.errors.iter().map(|e| e.line).collect::<Vec<_>>(), [4, 5]);
        let mut h = TestHost::default();
        let p = Params::new("");
        let other = list.command("Hello", "forsen").unwrap();
        assert_eq!(run(&other, &p, &mut h).unwrap(), "Hello World!");
        let own = list.command("hello", "joshimuz").unwrap();
        assert_eq!(run(&own, &p, &mut h).unwrap(), "Hello Joshimuz Chat!");
        assert!(list.replacement("_m", "x").is_some());
        assert_eq!(list.names("x"), ["hello"]);
    }

    #[test]
    fn chain_bodies_escape_replacements() {
        let body = parse("/chain /echo $1 | /echo b").unwrap();
        let out = run(&body, &Params::new("a|z"), &mut TestHost::default()).unwrap();
        assert_eq!(out, "/chain /echo a||z | /echo b");
    }
}
