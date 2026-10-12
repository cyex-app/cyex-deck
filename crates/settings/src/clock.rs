use time::{Date, OffsetDateTime};

/// Source of "now", injectable so backup rotation can be tested.
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;

    fn today(&self) -> Date {
        self.now().date()
    }
}

/// Local wall clock, UTC if the local offset can't be read.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
    }
}
