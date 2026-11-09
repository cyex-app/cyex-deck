//! V110-F01..F03 The paint log: stderr and `logs/paint.log` in the local data folder.

const LOG_CAP: u64 = 256 * 1024;

/// One line to stderr and `logs\paint.log` (the old half moves to `paint.old.log`).
pub(super) fn log(msg: &str) {
    eprintln!("paint: {msg}");
    let Some(dir) = crate::crash::logs_dir() else {
        return;
    };
    let path = dir.join("paint.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > LOG_CAP) {
        let _ = std::fs::rename(&path, dir.join("paint.old.log"));
    }
    let _ = std::fs::create_dir_all(dir);
    let stamp = stamp(time::OffsetDateTime::now_utc());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = writeln!(f, "{stamp} {msg}");
    }
}

/// `yyyy/mm/dd HH:MM:SS` UTC.
fn stamp(t: time::OffsetDateTime) -> String {
    format!(
        "{:04}/{:02}/{:02} {:02}:{:02}:{:02} UTC",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_read_like_the_app() {
        let t = time::macros::datetime!(2026-11-08 07:05:09 UTC);
        assert_eq!(stamp(t), "2026/11/08 07:05:09 UTC");
    }
}
