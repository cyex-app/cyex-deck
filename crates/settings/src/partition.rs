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
    Player,
    Highlights,
    /// V104-01
    Mentions,
    Commands,
    Hotkeys,
    Tabs,
    Browse,
    Emotes,
    Accounts,
    Miner,
    Modview,
    /// One lowercase word like the other files.
    #[serde(rename = "streammanager")]
    StreamManager,
    Notifications,
    Windows,
    Plugins,
    /// V105-11
    Uploader,
}

impl Partition {
    pub const ALL: [Partition; 19] = [
        Partition::General,
        Partition::Appearance,
        Partition::Chat,
        Partition::Player,
        Partition::Highlights,
        Partition::Mentions,
        Partition::Commands,
        Partition::Hotkeys,
        Partition::Tabs,
        Partition::Browse,
        Partition::Emotes,
        Partition::Accounts,
        Partition::Miner,
        Partition::Modview,
        Partition::StreamManager,
        Partition::Notifications,
        Partition::Windows,
        Partition::Plugins,
        Partition::Uploader,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Partition::General => "general",
            Partition::Appearance => "appearance",
            Partition::Chat => "chat",
            Partition::Player => "player",
            Partition::Highlights => "highlights",
            Partition::Mentions => "mentions",
            Partition::Commands => "commands",
            Partition::Hotkeys => "hotkeys",
            Partition::Tabs => "tabs",
            Partition::Browse => "browse",
            Partition::Emotes => "emotes",
            Partition::Accounts => "accounts",
            Partition::Miner => "miner",
            Partition::Modview => "modview",
            Partition::StreamManager => "streammanager",
            Partition::Notifications => "notifications",
            Partition::Windows => "windows",
            Partition::Plugins => "plugins",
            Partition::Uploader => "uploader",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == key)
    }

    /// Accounts point at tokens in this machine's credential store, and plugins' settings
    /// can hold their keys, and so can the image uploader's URL and headers (V105-11), so all three stay out of exports and share codes. Backups still
    /// include them.
    pub fn is_portable(self) -> bool {
        !matches!(self, Partition::Accounts | Partition::Plugins | Partition::Uploader)
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
            Partition::Player => roundtrip::<schema::Player>(value),
            Partition::Highlights => roundtrip::<schema::Highlights>(value),
            Partition::Mentions => roundtrip::<schema::Mentions>(value),
            Partition::Commands => roundtrip::<schema::Commands>(value),
            Partition::Hotkeys => roundtrip::<schema::Hotkeys>(value),
            Partition::Tabs => roundtrip::<schema::Tabs>(value),
            Partition::Browse => roundtrip::<schema::Browse>(value),
            Partition::Emotes => roundtrip::<schema::Emotes>(value),
            Partition::Accounts => roundtrip::<schema::Accounts>(value),
            Partition::Miner => roundtrip::<schema::Miner>(value),
            Partition::Modview => roundtrip::<schema::Modview>(value),
            Partition::StreamManager => roundtrip::<schema::StreamManager>(value),
            Partition::Notifications => roundtrip::<schema::Notifications>(value),
            Partition::Windows => roundtrip::<schema::Windows>(value),
            Partition::Plugins => roundtrip::<schema::Plugins>(value),
            Partition::Uploader => roundtrip::<schema::Uploader>(value),
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
    fn exports_leave_out_keys_and_credentials() {
        // V105-11 The uploader's URL and headers can hold keys.
        assert!(!Partition::Uploader.is_portable());
        assert!(!Partition::Accounts.is_portable());
        assert!(Partition::Chat.is_portable());
    }

    #[test]
    fn every_partition_has_a_default() {
        for p in Partition::ALL {
            assert!(p.default_value().is_object(), "{p:?}");
        }
    }
}
