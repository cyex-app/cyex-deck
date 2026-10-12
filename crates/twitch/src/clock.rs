//! UTC timestamps as Twitch writes them, without a date library.

/// Unix seconds as `2026-10-10T08:59:33Z`.
pub fn rfc3339(secs: i64) -> String {
    let (y, m, d, rest) = civil(secs);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// UTC now as `2026-10-10T08:59:33.045Z`; Hermes wants the milliseconds.
pub fn now_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let (y, m, d, rest) = civil(now.as_secs() as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60,
        now.subsec_millis()
    )
}

/// Year, month, day and seconds into the day (Howard Hinnant's days-to-civil).
fn civil(secs: i64) -> (i64, i64, i64, i64) {
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_760_090_400), "2025-10-10T10:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        let t = now_iso();
        assert_eq!((t.len(), &t[4..5], &t[10..11], &t[19..20]), (24, "-", "T", "."));
    }
}
