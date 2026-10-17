//! V104-01, V104-02 The v1.04/v1.05 `mentions` partition, read only (V106-33).

use serde::Deserialize;

use crate::schema::RegexFlavor;

pub(crate) const FILE: &str = "mentions.json";
pub(crate) const KEY: &str = "mentions";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct OldAlert {
    pub show: bool,
    pub flash: bool,
    pub notify: bool,
    pub sound: bool,
    pub sound_file: String,
    pub color: String,
}

impl Default for OldAlert {
    fn default() -> Self {
        Self {
            show: true,
            flash: false,
            notify: false,
            sound: false,
            sound_file: String::new(),
            color: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct OldYourName {
    pub enabled: bool,
    #[serde(flatten)]
    pub alert: OldAlert,
}

impl Default for OldYourName {
    fn default() -> Self {
        Self {
            enabled: true,
            alert: OldAlert::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct OldMentionRule {
    pub enabled: bool,
    pub pattern: String,
    pub whole_word: bool,
    pub case_sensitive: bool,
    pub lookalikes: bool,
    pub regex: bool,
    pub flavor: RegexFlavor,
    #[serde(flatten)]
    pub alert: OldAlert,
}

impl Default for OldMentionRule {
    fn default() -> Self {
        Self {
            enabled: true,
            pattern: String::new(),
            whole_word: true,
            case_sensitive: false,
            lookalikes: false,
            regex: false,
            flavor: RegexFlavor::Rust,
            alert: OldAlert::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct OldMentions {
    pub me: OldYourName,
    pub list: Vec<OldMentionRule>,
    pub global: bool,
    pub global_list: Vec<OldMentionRule>,
}
