//! V108-N25, V108-N10 What the room knows of its moments and its message rate. The counting
//! itself lives in `crate::moments`.

use super::Room;
use crate::moments::{Moment, MomentKind, Rate};

impl Room {
    /// Moments found since the last call.
    pub fn take_moments(&mut self) -> Vec<Moment> {
        self.moments.take_fresh()
    }

    /// Every kept moment, oldest first.
    pub fn moments(&self) -> Vec<Moment> {
        self.moments.list()
    }

    /// Messages per second now.
    pub fn rate(&self, now_ms: i64) -> Rate {
        self.moments.rate(now_ms)
    }

    /// A moment that isn't a chat line (hype train, title or category change).
    pub fn moment(&mut self, kind: MomentKind, label: &str, now_ms: i64) -> bool {
        self.moments.event(kind, label, 0, now_ms)
    }
}
