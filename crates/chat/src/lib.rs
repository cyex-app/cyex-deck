//! Twitch chat. Phase 2 only has the join budget; IRC arrives in phase 3.

mod budget;

pub use budget::{ConnectionCount, JoinBudget, ROOM_LIMIT};
