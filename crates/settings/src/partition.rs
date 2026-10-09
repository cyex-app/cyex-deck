use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::schema::{self, Schema};

/// One settings file each. Export and import work per partition (2.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Partition {
    General,
    Appearance,
    Chat,
    Highlights,
    Commands,
    Hotkeys,
    Tabs,
    Accounts,
    Miner,
    Modview,
    Notifications,
    Windows,
}

impl Partition {
    pub const ALL: [Partition; 12] = [
        Partition::General,
        Partition::Appearance,
        Partition::Chat,
        Partition::Highlights,
        Partition::Commands,
        Partition::Hotkeys,
        Partition::Tabs,
        Partition::Accounts,
        Partition::Miner,
        Partition::Modview,
        Partition::Notifications,
        Partition::Windows,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Partition::General => "general",
            Partition::Appearance => "appearance",
            Partition::Chat => "chat",
            Partition::Highlights => "highlights",
            Partition::Commands => "commands",
            Partition::Hotkeys => "hotkeys",
            Partition::Tabs => "tabs",
            Partition::Accounts => "accounts",
            Partition::Miner => "miner",
            Partition::Modview => "modview",
            Partition::Notifications => "notifications",
            Partition::Windows => "windows",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == key)
    }

    pub fn file_name(self) -> String {
        format!("{}.json", self.key())
    }

    /// Runs a value through this partition's schema: fills missing fields with defaults,
    /// drops unknown ones, clamps out-of-range values. Fails on wrong types.
    pub(crate) fn normalize(self, value: Value) -> serde_json::Result<Value> {
        match self {
            Partition::General => roundtrip::<schema::General>(value),
            Partition::Appearance => roundtrip::<schema::Appearance>(value),
            Partition::Chat => roundtrip::<schema::Chat>(value),
            Partition::Highlights => roundtrip::<schema::Highlights>(value),
            Partition::Commands => roundtrip::<schema::Commands>(value),
            Partition::Hotkeys => roundtrip::<schema::Hotkeys>(value),
            Partition::Tabs => roundtrip::<schema::Tabs>(value),
            Partition::Accounts => roundtrip::<schema::Accounts>(value),
            Partition::Miner => roundtrip::<schema::Miner>(value),
            Partition::Modview => roundtrip::<schema::Modview>(value),
            Partition::Notifications => roundtrip::<schema::Notifications>(value),
            Partition::Windows => roundtrip::<schema::Windows>(value),
        }
    }

    pub(crate) fn default_value(self) -> Value {
        self.normalize(Value::Object(Default::default()))
            .expect("every schema accepts an empty object")
    }
}

fn roundtrip<T: Schema>(value: Value) -> serde_json::Result<Value> {
    let mut typed: T = serde_json::from_value(value)?;
    typed.sanitize();
    serde_json::to_value(typed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_and_match_serde() {
        for p in Partition::ALL {
            assert_eq!(Partition::from_key(p.key()), Some(p));
            assert_eq!(serde_json::to_value(p).unwrap(), Value::String(p.key().into()));
        }
        assert_eq!(Partition::from_key("nope"), None);
    }

    #[test]
    fn every_partition_has_a_default() {
        for p in Partition::ALL {
            assert!(p.default_value().is_object(), "{p:?}");
        }
    }
}
