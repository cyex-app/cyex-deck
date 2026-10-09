use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use time::{Date, Duration, Month, OffsetDateTime, Time};

use crate::Clock;

/// `2026/10/<d>`.
pub(crate) fn day(d: u8) -> Date {
    Date::from_calendar_date(2026, Month::October, d).unwrap()
}

/// A fresh folder under the system temp dir, removed on drop.
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cyex-test-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Clock the test moves by hand.
pub(crate) struct FakeClock(Mutex<OffsetDateTime>);

impl FakeClock {
    pub(crate) fn at(date: Date) -> Self {
        Self(Mutex::new(
            date.with_time(Time::from_hms(12, 0, 0).unwrap()).assume_utc(),
        ))
    }

    pub(crate) fn advance_days(&self, days: i64) {
        *self.0.lock().unwrap() += Duration::days(days);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().unwrap()
    }
}
