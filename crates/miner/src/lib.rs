//! The points miner's logic (08), a Rust port of twitch-channel-points-miner-v2: which
//! channels get the two watch slots, watch streaks, prediction bets, what Hermes says,
//! community goals and the stats log. No I/O here; `app/src/miner` runs it.

pub mod bet;
pub mod events;
pub mod slots;
pub mod stats;

/// W2 Points to put toward a goal now: what's still needed, what this stream still
/// allows, and what the balance holds, whichever is least.
pub fn goal_share(
    needed: u64,
    contributed: u64,
    per_stream_max: u64,
    mine_this_stream: u64,
    balance: u64,
) -> u64 {
    needed
        .saturating_sub(contributed)
        .min(per_stream_max.saturating_sub(mine_this_stream))
        .min(balance)
}

#[cfg(test)]
mod tests {
    #[test]
    fn goal_share_takes_the_least() {
        assert_eq!(super::goal_share(9000, 5000, 2000, 300, 10_000), 1700);
        assert_eq!(super::goal_share(9000, 5000, 2000, 300, 100), 100);
        assert_eq!(super::goal_share(9000, 9500, 2000, 0, 100), 0);
    }
}
