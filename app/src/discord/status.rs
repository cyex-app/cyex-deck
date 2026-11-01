//! V108-N26 What the settings row says about the Discord link. Set by the worker thread,
//! read by `discord_status`. Pure apart from the one atomic.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::Serialize;
use ts_rs::TS;

/// The link to the Discord client while "Show on Discord" is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DiscordStatus {
    /// The setting is off, or nothing has been tried yet.
    Off,
    /// The last handshake worked.
    Connected,
    /// No pipe could be opened, or Discord stopped answering.
    NotRunning,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

impl DiscordStatus {
    fn code(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Connected => 1,
            Self::NotRunning => 2,
        }
    }

    fn from_code(c: u8) -> Self {
        match c {
            1 => Self::Connected,
            2 => Self::NotRunning,
            _ => Self::Off,
        }
    }

    /// The result of a connect attempt.
    pub fn of_connect(connected: bool) -> Self {
        if connected { Self::Connected } else { Self::NotRunning }
    }
}

pub fn set(s: DiscordStatus) {
    CURRENT.store(s.code(), Ordering::Relaxed);
}

pub fn get() -> DiscordStatus {
    DiscordStatus::from_code(CURRENT.load(Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for s in [DiscordStatus::Off, DiscordStatus::Connected, DiscordStatus::NotRunning] {
            assert_eq!(DiscordStatus::from_code(s.code()), s);
        }
        assert_eq!(DiscordStatus::from_code(99), DiscordStatus::Off);
    }

    #[test]
    fn a_connect_result_maps_to_a_status() {
        assert_eq!(DiscordStatus::of_connect(true), DiscordStatus::Connected);
        assert_eq!(DiscordStatus::of_connect(false), DiscordStatus::NotRunning);
    }
}
