//! V107-R9b Twitch writes dates into bell texts its own way ("November 10, 2026 7:59:00 am
//! UTC"). Toasts show them as `yyyy/mm/dd hh:mm` in local time (3.1, 3.2); same rules as
//! `ui/src/features/bell/dates.ts`. A time without a zone is UTC. A date alone stays that day.

use std::sync::LazyLock;

use regex::{Captures, Regex};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

const MONTH: &str = r"(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|june?|july?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\.?";
const TIME: &str = r"(?:,?\s+(?:at\s+)?(\d{1,2}):(\d{2})(?::(\d{2}))?(?:\s*([ap])\.?m\.?)?)?";

fn zone(sep: &str) -> String {
    format!(r"(?:{sep}(utc|gmt|z|[+-]\d{{2}}:?\d{{2}})\b)?")
}

static MONTH_FIRST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i)\b{MONTH}\s+(\d{{1,2}})(?:st|nd|rd|th)?,?\s+(\d{{4}})\b{TIME}{}", zone(r"\s*"))).unwrap()
});
static DAY_FIRST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i)\b(\d{{1,2}})(?:st|nd|rd|th)?\s+{MONTH},?\s+(\d{{4}})\b{TIME}{}", zone(r"\s*"))).unwrap()
});
static ISO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)\b(\d{{4}})-(\d{{2}})-(\d{{2}})(?:[T ](\d{{2}}):(\d{{2}})(?::(\d{{2}})(?:\.\d+)?)?)?{}",
        zone(r"\s?")
    ))
    .unwrap()
});

const MONTHS: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];

struct Parts<'a> {
    y: &'a str,
    mo: Option<usize>,
    d: &'a str,
    h: Option<&'a str>,
    mi: Option<&'a str>,
    s: Option<&'a str>,
    ap: Option<&'a str>,
    zone: Option<&'a str>,
}

fn num(s: Option<&str>) -> i64 {
    s.and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn render(p: &Parts, raw: &str, offset: &dyn Fn(OffsetDateTime) -> UtcOffset) -> String {
    let (Some(mo), Ok(y), Ok(d)) = (p.mo, p.y.parse::<i32>(), p.d.parse::<u8>()) else {
        return raw.into();
    };
    // V107-N9a Month 13 or Feb 31 isn't a date, with or without a time: leave the text.
    let Some(date) = u8::try_from(mo + 1)
        .ok()
        .and_then(|m| Month::try_from(m).ok())
        .and_then(|m| Date::from_calendar_date(y, m, d).ok())
    else {
        return raw.into();
    };
    let Some(h) = p.h else {
        return format!("{y:04}/{:02}/{d:02}", mo + 1);
    };
    let mut h = num(Some(h));
    if let Some(ap) = p.ap {
        if !(1..=12).contains(&h) {
            return raw.into();
        }
        h = (h % 12) + if ap.eq_ignore_ascii_case("p") { 12 } else { 0 };
    }
    let (mi, s) = (num(p.mi), num(p.s));
    if h > 23 || mi > 59 || s > 59 {
        return raw.into();
    }
    let Ok(t) = Time::from_hms(h as u8, mi as u8, s as u8) else {
        return raw.into();
    };
    let mut at = PrimitiveDateTime::new(date, t).assume_utc();
    if let Some(z) = p.zone {
        let b = z.as_bytes();
        if b[0] == b'+' || b[0] == b'-' {
            let digits: String = z[1..].chars().filter(char::is_ascii_digit).collect();
            let mins = num(digits.get(..2)) * 60 + num(digits.get(2..));
            let sign = if b[0] == b'-' { -1 } else { 1 };
            at -= time::Duration::minutes(sign * mins);
        }
    }
    let at = at.to_offset(offset(at));
    format!("{:04}/{:02}/{:02} {:02}:{:02}", at.year(), u8::from(at.month()), at.day(), at.hour(), at.minute())
}

fn month(name: &str) -> Option<usize> {
    let n = name.get(..3)?.to_ascii_lowercase();
    MONTHS.iter().position(|m| *m == n)
}

fn g<'a>(c: &Captures<'a>, i: usize) -> Option<&'a str> {
    c.get(i).map(|m| m.as_str())
}

fn convert(text: &str, offset: &dyn Fn(OffsetDateTime) -> UtcOffset) -> String {
    let digits = text.as_bytes().windows(4).any(|w| w.iter().all(u8::is_ascii_digit));
    if !digits {
        return text.into();
    }
    let a = MONTH_FIRST.replace_all(text, |c: &Captures| {
        let p = Parts { y: g(c, 3).unwrap_or(""), mo: g(c, 1).and_then(month), d: g(c, 2).unwrap_or(""), h: g(c, 4), mi: g(c, 5), s: g(c, 6), ap: g(c, 7), zone: g(c, 8) };
        render(&p, &c[0], offset)
    });
    let b = DAY_FIRST.replace_all(&a, |c: &Captures| {
        let p = Parts { y: g(c, 3).unwrap_or(""), mo: g(c, 2).and_then(month), d: g(c, 1).unwrap_or(""), h: g(c, 4), mi: g(c, 5), s: g(c, 6), ap: g(c, 7), zone: g(c, 8) };
        render(&p, &c[0], offset)
    });
    ISO.replace_all(&b, |c: &Captures| {
        let p = Parts { y: g(c, 1).unwrap_or(""), mo: num(g(c, 2)).checked_sub(1).map(|m| m as usize), d: g(c, 3).unwrap_or(""), h: g(c, 4), mi: g(c, 5), s: g(c, 6), ap: None, zone: g(c, 7) };
        render(&p, &c[0], offset)
    })
    .into_owned()
}

/// Every date in `text` in the app's format and local time; text without dates stays.
pub fn local_dates(text: &str) -> String {
    convert(text, &|at| UtcOffset::local_offset_at(at).unwrap_or(UtcOffset::UTC))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Berlin: UTC+1 in winter, UTC+2 from April to October (close enough for the cases).
    fn berlin(at: OffsetDateTime) -> UtcOffset {
        let h = if (4..=10).contains(&u8::from(at.month())) { 2 } else { 1 };
        UtcOffset::from_hms(h, 0, 0).unwrap()
    }
    fn l(t: &str) -> String {
        convert(t, &berlin)
    }

    #[test]
    fn month_name_dates() {
        assert_eq!(l("Your sub ends on November 10, 2026 7:59:00 am UTC."), "Your sub ends on 2026/11/10 08:59.");
        assert_eq!(l("Starts Nov 10, 2026 at 7:59 PM UTC"), "Starts 2026/11/10 20:59");
        assert_eq!(l("Ends July 1st, 2026, 11:30 p.m. UTC!"), "Ends 2026/07/02 01:30!");
        assert_eq!(l("until Dec 31, 2026 11:00 pm GMT"), "until 2027/01/01 00:00");
        assert_eq!(l("12:05 am: January 5, 2026 12:05 AM UTC"), "12:05 am: 2026/01/05 01:05");
    }

    #[test]
    fn day_first_and_date_only() {
        assert_eq!(l("on 10 November 2026, 19:59 UTC"), "on 2026/11/10 20:59");
        assert_eq!(l("Renews on November 10, 2026."), "Renews on 2026/11/10.");
        assert_eq!(l("**xqc** gifted a sub on Sept 3, 2026"), "**xqc** gifted a sub on 2026/09/03");
    }

    #[test]
    fn iso_dates() {
        assert_eq!(l("at 2026-11-10T07:59:00Z"), "at 2026/11/10 08:59");
        assert_eq!(l("at 2026-11-10T07:59:00.123Z."), "at 2026/11/10 08:59.");
        assert_eq!(l("at 2026-07-01 10:00 UTC"), "at 2026/07/01 12:00");
        assert_eq!(l("at 2026-11-10T07:59:00-05:00"), "at 2026/11/10 13:59");
        assert_eq!(l("on 2026-11-10"), "on 2026/11/10");
    }

    #[test]
    fn text_without_dates_stays() {
        for t in [
            "**xqc** went live: Just Chatting",
            "Watch 2026 hours of streams",
            "May the best team win, 10 to 3",
            "Score 13:45 in round 2",
            "Nov 99, 2026 is not a day",
            "Feb 31, 2026 is not a day",
            "on 2026-13-05",
            "at 2026-02-30T10:00:00Z",
            "",
        ] {
            assert_eq!(l(t), t);
        }
    }
}
