use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::Schema;

const MAX_PLUGINS: usize = 100;
const MAX_GRANTED: usize = 20;
const MAX_VALUES: usize = 30;
const MAX_TEXT: usize = 500;

/// S5 Lua plugins. Off until the user turns them on; each plugin also needs its own OK.
/// Plugins themselves are folders, not settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Plugins {
    pub enabled: bool,
    /// By folder name.
    pub list: BTreeMap<String, PluginEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct PluginEntry {
    pub on: bool,
    /// The permissions the user agreed to, as the plugins crate writes them.
    pub granted: Vec<String>,
    /// The plugin's own settings, checked against its info file when it starts.
    #[ts(type = "Record<string, unknown>")]
    pub values: BTreeMap<String, Value>,
}

impl Schema for Plugins {
    fn sanitize(&mut self) {
        let ok = |id: &str| {
            !id.is_empty()
                && id.len() <= 64
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        };
        self.list.retain(|id, _| ok(id));
        while self.list.len() > MAX_PLUGINS {
            self.list.pop_last();
        }
        for e in self.list.values_mut() {
            e.granted.truncate(MAX_GRANTED);
            e.granted.retain(|g| g.len() <= 2000);
            e.values.retain(|k, v| {
                k.len() <= 40
                    && match v {
                        Value::String(s) => s.chars().count() <= MAX_TEXT,
                        Value::Bool(_) | Value::Number(_) => true,
                        _ => false,
                    }
            });
            while e.values.len() > MAX_VALUES {
                e.values.pop_last();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_only_plain_values_and_folder_names() {
        let mut p: Plugins = serde_json::from_value(json!({
            "enabled": true,
            "list": {
                "words": { "on": true, "granted": ["ReadChat"], "values": { "a": 1, "b": "x", "c": [1], "d": { "e": 1 } } },
                "../evil": { "on": true }
            }
        }))
        .unwrap();
        p.sanitize();
        assert_eq!(p.list.keys().collect::<Vec<_>>(), ["words"]);
        assert_eq!(p.list["words"].values.keys().collect::<Vec<_>>(), ["a", "b"]);
    }
}
