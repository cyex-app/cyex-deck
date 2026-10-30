//! V108-N03 A channel's schedule as an iCalendar file. Pure: the caller hands in the
//! events and the stamp. Times are UTC, lines end in CRLF and fold at 75 bytes.

use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Duration, OffsetDateTime};

/// Length of a stream that has no end time.
const DEFAULT_LEN: Duration = Duration::hours(2);

/// One planned stream.
pub struct Event<'a> {
    pub uid: &'a str,
    pub title: &'a str,
    pub category: Option<&'a str>,
    /// RFC 3339.
    pub start: &'a str,
    /// RFC 3339.
    pub end: Option<&'a str>,
    pub cancelled: bool,
}

/// The calendar for `channel` (display name). Cancelled streams and ones with a
/// time that doesn't parse are left out.
pub fn calendar(channel: &str, events: &[Event], stamp: OffsetDateTime) -> String {
    let mut out = vec![
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//cyex deck//schedule//EN".to_owned(),
        "CALSCALE:GREGORIAN".to_owned(),
        format!("X-WR-CALNAME:{}", escape(&format!("{channel} on Twitch"))),
    ];
    for e in events.iter().filter(|e| !e.cancelled) {
        let Ok(start) = OffsetDateTime::parse(e.start, &Rfc3339) else { continue };
        let end = e
            .end
            .and_then(|t| OffsetDateTime::parse(t, &Rfc3339).ok())
            .filter(|end| *end > start)
            .unwrap_or(start + DEFAULT_LEN);
        let title = if e.title.trim().is_empty() {
            format!("{channel} streams")
        } else {
            format!("{channel}: {}", e.title.trim())
        };
        out.push("BEGIN:VEVENT".to_owned());
        out.push(format!("UID:{}@cyex-deck", escape(e.uid)));
        out.push(format!("DTSTAMP:{}", utc(stamp)));
        out.push(format!("DTSTART:{}", utc(start)));
        out.push(format!("DTEND:{}", utc(end)));
        out.push(format!("SUMMARY:{}", escape(&title)));
        if let Some(c) = e.category.filter(|c| !c.is_empty()) {
            out.push(format!("DESCRIPTION:{}", escape(c)));
        }
        out.push(format!("URL:https://www.twitch.tv/{}", url_safe(channel)));
        out.push("END:VEVENT".to_owned());
    }
    out.push("END:VCALENDAR".to_owned());
    let mut text = String::new();
    for line in out {
        fold(&line, &mut text);
    }
    text
}

fn utc(t: OffsetDateTime) -> String {
    t.to_offset(time::UtcOffset::UTC)
        .format(format_description!("[year][month][day]T[hour][minute][second]Z"))
        .unwrap_or_default()
}

/// RFC 5545 text escaping.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out
}

/// Login-like text for the URL.
fn url_safe(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Appends `line` folded at 75 bytes (never inside a character), each line ending CRLF.
fn fold(line: &str, out: &mut String) {
    let mut len = 0;
    for c in line.chars() {
        let w = c.len_utf8();
        if len + w > 75 {
            out.push_str("\r\n ");
            len = 1;
        }
        out.push(c);
        len += w;
    }
    out.push_str("\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp() -> OffsetDateTime {
        OffsetDateTime::parse("2026-10-30T12:00:00Z", &Rfc3339).unwrap()
    }

    fn ev<'a>(uid: &'a str, title: &'a str, start: &'a str, end: Option<&'a str>, cancelled: bool) -> Event<'a> {
        Event { uid, title, category: Some("Just Chatting"), start, end, cancelled }
    }

    #[test]
    fn times_are_utc_and_lines_end_in_crlf() {
        let s = calendar(
            "Xx",
            &[ev("a", "Ranked", "2026-10-30T20:00:00+02:00", Some("2026-10-30T22:30:00Z"), false)],
            stamp(),
        );
        assert!(s.starts_with("BEGIN:VCALENDAR\r\n") && s.ends_with("END:VCALENDAR\r\n"));
        assert!(s.contains("DTSTART:20261030T180000Z\r\n"));
        assert!(s.contains("DTEND:20261030T223000Z\r\n"));
        assert!(s.contains("DTSTAMP:20261030T120000Z\r\n"));
        assert!(s.contains("SUMMARY:Xx: Ranked\r\n"));
        assert!(!s.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn cancelled_and_bad_times_are_skipped() {
        let s = calendar(
            "Xx",
            &[
                ev("a", "", "2026-10-30T18:00:00Z", None, true),
                ev("b", "", "soon", None, false),
                ev("c", "", "2026-10-31T18:00:00Z", None, false),
            ],
            stamp(),
        );
        assert_eq!(s.matches("BEGIN:VEVENT").count(), 1);
        assert!(s.contains("UID:c@cyex-deck"));
        // No end: two hours; no title: the channel.
        assert!(s.contains("DTEND:20261031T200000Z") && s.contains("SUMMARY:Xx streams"));
    }

    #[test]
    fn an_end_before_the_start_falls_back_to_the_default_length() {
        let s = calendar("Xx", &[ev("a", "T", "2026-10-30T18:00:00Z", Some("2026-10-30T17:00:00Z"), false)], stamp());
        assert!(s.contains("DTEND:20261030T200000Z"));
    }

    #[test]
    fn text_is_escaped() {
        assert_eq!(escape("a,b;c\\d\ne\r"), "a\\,b\\;c\\\\d\\ne");
        let s = calendar("Xx", &[ev("a", "Rank, pt 2; late", "2026-10-30T18:00:00Z", None, false)], stamp());
        assert!(s.contains("SUMMARY:Xx: Rank\\, pt 2\\; late\r\n"));
    }

    #[test]
    fn long_lines_fold_on_character_edges() {
        let long = "é".repeat(60);
        let s = calendar("Xx", &[ev("a", &long, "2026-10-30T18:00:00Z", None, false)], stamp());
        for line in s.split("\r\n") {
            assert!(line.len() <= 75);
        }
        let unfolded = s.replace("\r\n ", "");
        assert!(unfolded.contains(&format!("SUMMARY:Xx: {long}")));
    }
}
