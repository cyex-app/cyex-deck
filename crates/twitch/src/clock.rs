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

/// Unix seconds from `2026-10-10T08:59:33Z` (a fraction and a `Z` or `+00:00` are fine);
/// `None` for anything else.
pub fn parse_rfc3339(s: &str) -> Option<i64> {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b'T' | b' ') || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, m, d, h, mi, sec) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    let tail = &s[19..];
    let tail = tail.strip_prefix('.').map_or(tail, |f| f.trim_start_matches(|c: char| c.is_ascii_digit()));
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    if !matches!(tail, "" | "Z" | "+00:00" | "-00:00") {
        return None;
    }
    // Days from civil (Howard Hinnant).
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + h * 3600 + mi * 60 + sec)
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
        for secs in [0, 1_760_090_400, 951_782_400, 1_900_000_001] {
            assert_eq!(parse_rfc3339(&rfc3339(secs)), Some(secs));
        }
        assert_eq!(parse_rfc3339("2025-10-10T10:00:00.123Z"), Some(1_760_090_400));
        assert_eq!(parse_rfc3339("2025-10-10T10:00:00+02:00"), None);
        assert_eq!(parse_rfc3339("soon"), None);
        let t = now_iso();
        assert_eq!((t.len(), &t[4..5], &t[10..11], &t[19..20]), (24, "-", "T", "."));
    }
}
