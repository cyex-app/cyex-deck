//! S5 A plugin's `info.json`: who made it, what it may do, the settings it shows.
//! Close to Chatterino's file, so their plugins need few changes; unknown fields are
//! ignored, an unknown permission is an error (it can't be granted).

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

pub const INFO: &str = "info.json";
pub const ENTRY: &str = "init.lua";
/// Where FilesystemRead and FilesystemWrite apply.
pub const DATA: &str = "data";

const MAX_FIELDS: usize = 30;
const MAX_CHOICES: usize = 50;
pub const MAX_TEXT: usize = 500;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "PluginManifest")]
pub struct Manifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub version: String,
    #[serde(default)]
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub settings: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "type")]
#[ts(export, rename = "PluginPermission")]
pub enum Permission {
    /// Lines in open chats, as they arrive.
    ReadChat,
    /// Change or hide lines before they show. Reads them too.
    ChangeChat,
    /// Send chat messages as the user, and change or stop the ones the user sends.
    SendMessages,
    /// HTTP to these hosts; `*.example.com` takes in its subdomains. Empty: any website.
    Network {
        #[serde(default)]
        hosts: Vec<String>,
    },
    /// Files in the plugin's own `data` folder.
    FilesystemRead,
    FilesystemWrite,
}

impl Permission {
    /// What the user agrees to, as stored in settings. A plugin that asks for more after
    /// an update waits for the user's OK again.
    pub fn key(&self) -> String {
        match self {
            Permission::ReadChat => "ReadChat".into(),
            Permission::ChangeChat => "ChangeChat".into(),
            Permission::SendMessages => "SendMessages".into(),
            Permission::Network { hosts } => {
                let mut hosts: Vec<String> = hosts.iter().map(|h| h.to_ascii_lowercase()).collect();
                hosts.sort();
                hosts.dedup();
                format!("Network:{}", hosts.join(","))
            }
            Permission::FilesystemRead => "FilesystemRead".into(),
            Permission::FilesystemWrite => "FilesystemWrite".into(),
        }
    }
}

/// A setting the plugin asks for; Settings → Plugins draws it.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "PluginField")]
pub struct Field {
    pub key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hint: Option<String>,
    #[serde(flatten)]
    pub kind: FieldKind,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, rename = "PluginFieldKind")]
pub enum FieldKind {
    Toggle {
        #[serde(default)]
        default: bool,
    },
    Text {
        #[serde(default)]
        default: String,
    },
    Number {
        #[serde(default)]
        default: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        max: Option<f64>,
    },
    Choice {
        options: Vec<Choice>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        default: Option<String>,
    },
}

/// `"a"` or `{ "value": "a", "label": "Option A" }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, rename = "PluginChoice")]
pub struct Choice {
    pub value: String,
    pub label: String,
}

impl<'de> Deserialize<'de> for Choice {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Plain(String),
            Full { value: String, label: Option<String> },
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Plain(v) => Choice {
                label: v.clone(),
                value: v,
            },
            Raw::Full { value, label } => Choice {
                label: label.unwrap_or_else(|| value.clone()),
                value,
            },
        })
    }
}

impl Field {
    /// The stored value made to fit this field; the default when it doesn't.
    pub fn value(&self, stored: Option<&Value>) -> Value {
        match &self.kind {
            FieldKind::Toggle { default } => Value::Bool(stored.and_then(Value::as_bool).unwrap_or(*default)),
            FieldKind::Text { default } => {
                let s = stored.and_then(Value::as_str).unwrap_or(default);
                Value::String(s.chars().take(MAX_TEXT).collect())
            }
            FieldKind::Number { default, min, max } => {
                let mut n = stored
                    .and_then(Value::as_f64)
                    .filter(|n| n.is_finite())
                    .unwrap_or(*default);
                if let Some(min) = min {
                    n = n.max(*min);
                }
                if let Some(max) = max {
                    n = n.min(*max);
                }
                // Whole numbers stay integers in Lua.
                if n.fract() == 0.0 && n.abs() < 9e15 {
                    Value::from(n as i64)
                } else {
                    serde_json::Number::from_f64(n).map_or(Value::Null, Value::Number)
                }
            }
            FieldKind::Choice { options, default } => {
                let ok = |v: &str| options.iter().any(|o| o.value == v);
                let v = stored
                    .and_then(Value::as_str)
                    .filter(|v| ok(v))
                    .or(default.as_deref().filter(|v| ok(v)))
                    .or(options.first().map(|o| o.value.as_str()))
                    .unwrap_or("");
                Value::String(v.to_owned())
            }
        }
    }
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let m: Manifest = serde_json::from_str(text).map_err(|e| format!("{INFO}: {e}"))?;
        m.check()?;
        Ok(m)
    }

    pub fn load(dir: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(dir.join(INFO)).map_err(|_| format!("No {INFO}."))?;
        let m = Self::parse(&text)?;
        if !dir.join(ENTRY).is_file() {
            return Err(format!("No {ENTRY}."));
        }
        Ok(m)
    }

    pub fn has(&self, p: &Permission) -> bool {
        self.permissions.contains(p)
    }

    /// Keys of every permission, for comparing with what the user agreed to.
    pub fn keys(&self) -> Vec<String> {
        self.permissions.iter().map(Permission::key).collect()
    }

    fn check(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 60 {
            return Err("The name needs 1-60 characters.".into());
        }
        if self.version.trim().is_empty() || self.version.len() > 30 {
            return Err("The version needs 1-30 characters.".into());
        }
        if self.description.chars().count() > MAX_TEXT {
            return Err(format!("The description is longer than {MAX_TEXT} characters."));
        }
        if self.authors.len() > 10 || self.tags.len() > 10 {
            return Err("Up to 10 authors and 10 tags.".into());
        }
        if let Some(h) = &self.homepage
            && !(h.starts_with("https://") || h.starts_with("http://"))
        {
            return Err("The homepage must be a web link.".into());
        }
        for p in &self.permissions {
            if let Permission::Network { hosts } = p
                && let Some(bad) = hosts.iter().find(|h| !crate::perm::valid_host_pattern(h))
            {
                return Err(format!("Can't read the host \"{bad}\"."));
            }
        }
        if self.settings.len() > MAX_FIELDS {
            return Err(format!("Up to {MAX_FIELDS} settings."));
        }
        let mut keys = HashSet::new();
        for f in &self.settings {
            let k = &f.key;
            let ok =
                !k.is_empty() && k.len() <= 40 && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !ok || !keys.insert(k.as_str()) {
                return Err(format!(
                    "Setting key \"{k}\": letters, digits and _, each key once."
                ));
            }
            if f.label.trim().is_empty() || f.label.chars().count() > 60 {
                return Err(format!("Setting \"{k}\" needs a label of 1-60 characters."));
            }
            if let FieldKind::Choice { options, .. } = &f.kind
                && (options.is_empty() || options.len() > MAX_CHOICES)
            {
                return Err(format!("Setting \"{k}\" needs 1-{MAX_CHOICES} options."));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FULL: &str = r#"{
        "$schema": "https://example.com/schema.json",
        "name": "Counter",
        "description": "Counts words.",
        "authors": ["someone"],
        "version": "1.2.0",
        "license": "MIT",
        "homepage": "https://example.com",
        "tags": ["fun"],
        "permissions": [{"type": "ReadChat"}, {"type": "Network", "hosts": ["API.example.com", "*.example.org"]}],
        "settings": [
            {"key": "loud", "label": "Loud", "type": "toggle", "default": true},
            {"key": "limit", "label": "Limit", "type": "number", "default": 5, "min": 1, "max": 50},
            {"key": "mode", "label": "Mode", "type": "choice", "options": ["a", {"value": "b", "label": "Bee"}]}
        ]
    }"#;

    #[test]
    fn reads_chatterino_style_info() {
        let m = Manifest::parse(FULL).unwrap();
        assert_eq!(m.name, "Counter");
        assert!(m.has(&Permission::ReadChat));
        assert_eq!(m.keys(), ["ReadChat", "Network:*.example.org,api.example.com"]);
        let FieldKind::Choice { options, .. } = &m.settings[2].kind else {
            panic!()
        };
        assert_eq!(
            options[1],
            Choice {
                value: "b".into(),
                label: "Bee".into()
            }
        );
    }

    #[test]
    fn minimal_info_is_enough() {
        let m = Manifest::parse(r#"{"name": "x", "version": "1"}"#).unwrap();
        assert!(m.permissions.is_empty() && m.settings.is_empty());
    }

    #[test]
    fn refuses_what_it_cant_grant_or_show() {
        for bad in [
            r#"{"name": "x", "version": "1", "permissions": [{"type": "Everything"}]}"#,
            r#"{"name": "", "version": "1"}"#,
            r#"{"name": "x", "version": "1", "homepage": "javascript:alert(1)"}"#,
            r#"{"name": "x", "version": "1", "permissions": [{"type": "Network", "hosts": ["a b"]}]}"#,
            r#"{"name": "x", "version": "1", "settings": [{"key": "a-b", "label": "A", "type": "toggle"}]}"#,
            r#"{"name": "x", "version": "1", "settings": [{"key": "a", "label": "A", "type": "toggle"}, {"key": "a", "label": "B", "type": "text"}]}"#,
            r#"{"name": "x", "version": "1", "settings": [{"key": "a", "label": "A", "type": "choice", "options": []}]}"#,
        ] {
            assert!(Manifest::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn stored_values_are_made_to_fit() {
        let m = Manifest::parse(FULL).unwrap();
        let [loud, limit, mode] = &m.settings[..] else {
            panic!()
        };
        assert_eq!(loud.value(None), json!(true));
        assert_eq!(loud.value(Some(&json!("yes"))), json!(true));
        assert_eq!(limit.value(Some(&json!(500))), json!(50));
        assert_eq!(limit.value(Some(&json!(-1))), json!(1));
        assert_eq!(limit.value(Some(&json!(2.5))), json!(2.5));
        assert_eq!(mode.value(Some(&json!("zzz"))), json!("a"));
        assert_eq!(mode.value(Some(&json!("b"))), json!("b"));
    }
}
