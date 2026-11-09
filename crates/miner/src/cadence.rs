//! V110-U68 How busy the miner's loops are. With the app hidden (every window closed to
//! the tray) and nothing to earn, nothing needs a quick look: Hermes tells when a stream
//! goes up, and a slow poll catches what it missed. Shown, or busy, everything runs as usual.

use std::time::Duration;

/// Channel list poll.
pub const POLL: Duration = Duration::from_secs(60);
/// The slow poll while hidden and idle (the safety net under Hermes).
pub const POLL_HIDDEN: Duration = Duration::from_secs(5 * 60);
/// Inventory while something is being earned.
pub const INVENTORY_BUSY: Duration = Duration::from_secs(60);
/// Inventory otherwise.
pub const INVENTORY_IDLE: Duration = Duration::from_secs(15 * 60);
/// Inventory while hidden and idle.
pub const INVENTORY_HIDDEN: Duration = Duration::from_secs(30 * 60);
/// Campaign list refresh.
pub const CAMPAIGNS: Duration = Duration::from_secs(30 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cadence {
    /// Between channel list polls.
    pub poll: Duration,
    /// The watch loop sleeps until woken (a stream came up, a window showed).
    pub watch_park: bool,
    /// Between inventory reads.
    pub inventory: Duration,
    /// Between campaign list loads; `None` skips it (see `campaigns_every`).
    pub campaigns: Option<Duration>,
}

impl Cadence {
    /// The campaign interval, with badges wanted explicitly forcing the normal one.
    pub fn campaigns_every(&self, badges_picked: bool) -> Option<Duration> {
        self.campaigns.or(badges_picked.then_some(CAMPAIGNS))
    }
}

/// `visible`: some app window is shown. `busy`: the miner has an account and something to
/// do: a watch slot in use, the player counting as watching, or a badge focus.
pub fn cadence(visible: bool, busy: bool) -> Cadence {
    if !visible && !busy {
        return Cadence {
            poll: POLL_HIDDEN,
            watch_park: true,
            inventory: INVENTORY_HIDDEN,
            campaigns: None,
        };
    }
    Cadence {
        poll: POLL,
        watch_park: false,
        inventory: if busy { INVENTORY_BUSY } else { INVENTORY_IDLE },
        campaigns: Some(CAMPAIGNS),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_hidden_and_idle_slows_down() {
        let slow = cadence(false, false);
        assert_eq!(
            (slow.poll, slow.watch_park, slow.inventory, slow.campaigns),
            (POLL_HIDDEN, true, INVENTORY_HIDDEN, None)
        );
        for (visible, busy) in [(true, true), (true, false), (false, true)] {
            let c = cadence(visible, busy);
            assert_eq!(
                (c.poll, c.watch_park, c.campaigns),
                (POLL, false, Some(CAMPAIGNS)),
                "{visible} {busy}"
            );
        }
    }

    #[test]
    fn inventory_follows_busy() {
        assert_eq!(cadence(true, true).inventory, INVENTORY_BUSY);
        assert_eq!(cadence(false, true).inventory, INVENTORY_BUSY);
        assert_eq!(cadence(true, false).inventory, INVENTORY_IDLE);
    }

    #[test]
    fn picked_badges_keep_the_campaign_loop_going() {
        let idle = cadence(false, false);
        assert_eq!(idle.campaigns_every(false), None);
        assert_eq!(idle.campaigns_every(true), Some(CAMPAIGNS));
        assert_eq!(cadence(true, true).campaigns_every(false), Some(CAMPAIGNS));
    }
}
