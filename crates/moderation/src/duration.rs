//! Durations typed in mod commands: `600`, `10m`, `1h30m`, `2d`, `1w`, `1mo`. A bare
//! number counts in the command's own unit (seconds for timeouts, minutes for
//! followers-only).

const UNITS: [(&str, u64); 6] = [("mo", 2_592_000), ("w", 604_800), ("d", 86_400), ("h", 3_600), ("m", 60), ("s", 1)];

/// Seconds, or `None` when it doesn't read. `bare` is what a plain number counts as.
pub fn parse(text: &str, bare: u64) -> Option<u64> {
    let t = text.trim().to_ascii_lowercase();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<u64>() {
        return n.checked_mul(bare);
    }
    let mut total: u64 = 0;
    let mut rest = t.as_str();
    while !rest.is_empty() {
        let digits = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        if digits == 0 {
            return None;
        }
        let n: u64 = rest[..digits].parse().ok()?;
        rest = &rest[digits..];
        let (unit, size) = UNITS.iter().find(|(u, _)| rest.starts_with(u))?;
        rest = &rest[unit.len()..];
        total = total.checked_add(n.checked_mul(*size)?)?;
    }
    Some(total)
}

/// Whether a word looks like a duration, so commands can tell it from a reason.
pub fn looks_like(text: &str) -> bool {
    parse(text, 1).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_and_bare_numbers() {
        assert_eq!(parse("600", 1), Some(600));
        assert_eq!(parse("10", 60), Some(600));
        assert_eq!(parse("10m", 1), Some(600));
        assert_eq!(parse("1h30m", 1), Some(5_400));
        assert_eq!(parse("2D", 1), Some(172_800));
        assert_eq!(parse("1w", 1), Some(604_800));
        assert_eq!(parse("1mo", 1), Some(2_592_000));
        assert_eq!(parse("30s", 60), Some(30));
    }

    #[test]
    fn junk_is_refused() {
        for t in ["", "m", "10x", "abc", "1h3", "-5"] {
            assert_eq!(parse(t, 1), None, "{t}");
        }
        assert!(!looks_like("spam"));
        assert!(looks_like("5m"));
    }
}
