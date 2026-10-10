//! `$datetime(format, zone, locale, unix ms)`: Chatty's preset names or a Java-style
//! pattern (`yyyy-MM-dd HH:mm:ss`, `EEEE`, `MMMM`, `a`, `z`, `xx`, `'text'`). Zones are
//! UTC or an offset like `+02:00`; anything else uses local time. Output is English.

use time::{OffsetDateTime, UtcOffset};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

pub fn format(pattern: &str, zone: &str, at_ms: i64, local_offset_secs: i32) -> String {
    let offset = parse_zone(zone)
        .unwrap_or_else(|| UtcOffset::from_whole_seconds(local_offset_secs).unwrap_or(UtcOffset::UTC));
    let Ok(t) = OffsetDateTime::from_unix_timestamp_nanos(i128::from(at_ms) * 1_000_000) else {
        return String::new();
    };
    let t = t.to_offset(offset);
    let pattern = match pattern.trim() {
        "" => "EEEE, MMMM d, yyyy HH:mm:ss z",
        "date" => "yyyy-MM-dd",
        "date2" => "yyyy-MM-dd z 'GMT'xx",
        "datetime" => "yyyy-MM-dd HH:mm:ss",
        "datetime2" => "yyyy-MM-dd HH:mm:ss z 'GMT'xx",
        "time" => "HH:mm:ss",
        "time2" => "HH:mm:ss z 'GMT'xx",
        "time_short" => "HH:mm",
        "time_medium" => "HH:mm:ss",
        "time_long" | "time_full" => "HH:mm:ss z",
        "date_short" => "yyyy/MM/dd",
        "date_medium" => "MMM d, yyyy",
        "date_long" => "MMMM d, yyyy",
        "date_full" => "EEEE, MMMM d, yyyy",
        "datetime_short" => "yyyy/MM/dd HH:mm",
        "datetime_medium" => "MMM d, yyyy HH:mm:ss",
        "datetime_long" => "MMMM d, yyyy HH:mm:ss z",
        "datetime_full" => "EEEE, MMMM d, yyyy HH:mm:ss z",
        custom => custom,
    };
    render(pattern, &t)
}

/// `UTC`, `GMT`, `Z`, `+2`, `-05:30`, `GMT+1`, `UTC-3`.
fn parse_zone(zone: &str) -> Option<UtcOffset> {
    let z = zone.trim().to_ascii_uppercase();
    let rest = z
        .strip_prefix("GMT")
        .or_else(|| z.strip_prefix("UTC"))
        .unwrap_or(&z);
    if rest.is_empty() || rest == "Z" {
        return (!z.is_empty()).then_some(UtcOffset::UTC);
    }
    let (sign, digits) = match rest.as_bytes()[0] {
        b'+' => (1, &rest[1..]),
        b'-' => (-1, &rest[1..]),
        _ => return None,
    };
    let (h, m) = match digits.split_once(':') {
        Some((h, m)) => (h.parse::<i32>().ok()?, m.parse::<i32>().ok()?),
        None if digits.len() == 4 => (digits[..2].parse().ok()?, digits[2..].parse().ok()?),
        None => (digits.parse().ok()?, 0),
    };
    if h > 18 || m > 59 {
        return None;
    }
    UtcOffset::from_whole_seconds(sign * (h * 3600 + m * 60)).ok()
}

fn render(pattern: &str, t: &OffsetDateTime) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            i += 1;
            if chars.get(i) == Some(&'\'') {
                out.push('\'');
                i += 1;
                continue;
            }
            while i < chars.len() {
                if chars[i] == '\'' {
                    if chars.get(i + 1) == Some(&'\'') {
                        out.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                out.push(chars[i]);
                i += 1;
            }
            continue;
        }
        if !c.is_ascii_alphabetic() {
            out.push(c);
            i += 1;
            continue;
        }
        let mut n = 1;
        while chars.get(i + n) == Some(&c) {
            n += 1;
        }
        i += n;
        out.push_str(&field(c, n, t));
    }
    out
}

fn pad(v: i64, n: usize) -> String {
    format!("{v:0n$}")
}

fn field(c: char, n: usize, t: &OffsetDateTime) -> String {
    let hour = i64::from(t.hour());
    let offset = t.offset().whole_seconds();
    let (oh, om) = (offset.abs() / 3600, offset.abs() % 3600 / 60);
    let sign = if offset < 0 { '-' } else { '+' };
    match c {
        'y' | 'u' => {
            let y = i64::from(t.year());
            if n == 2 {
                pad(y.rem_euclid(100), 2)
            } else {
                pad(y, n)
            }
        }
        'M' | 'L' => {
            let m = u8::from(t.month()) as usize;
            match n {
                1 | 2 => pad(m as i64, n),
                3 => MONTHS[m - 1][..3].to_owned(),
                _ => MONTHS[m - 1].to_owned(),
            }
        }
        'd' => pad(i64::from(t.day()), n),
        'D' => pad(i64::from(t.ordinal()), n),
        'E' | 'e' | 'c' => {
            let d = DAYS[t.weekday().number_days_from_monday() as usize];
            if n >= 4 {
                d.to_owned()
            } else if n == 3 || c == 'E' {
                d[..3].to_owned()
            } else {
                pad(i64::from(t.weekday().number_from_monday()), n)
            }
        }
        'H' => pad(hour, n),
        'k' => pad(if hour == 0 { 24 } else { hour }, n),
        'h' => pad(if hour % 12 == 0 { 12 } else { hour % 12 }, n),
        'K' => pad(hour % 12, n),
        'm' => pad(i64::from(t.minute()), n),
        's' => pad(i64::from(t.second()), n),
        'S' => {
            let nanos = format!("{:09}", t.nanosecond());
            nanos[..n.min(9)].to_owned()
        }
        'a' => if hour < 12 { "AM" } else { "PM" }.to_owned(),
        'z' | 'O' | 'V' => {
            if offset == 0 {
                if n >= 4 {
                    "Coordinated Universal Time".into()
                } else {
                    "UTC".into()
                }
            } else if om == 0 {
                format!("GMT{sign}{oh}")
            } else {
                format!("GMT{sign}{oh}:{om:02}")
            }
        }
        'Z' | 'x' | 'X' => {
            if c == 'X' && offset == 0 {
                return "Z".into();
            }
            match n {
                1 if om == 0 => format!("{sign}{oh:02}"),
                1 | 2 => format!("{sign}{oh:02}{om:02}"),
                _ if c == 'Z' => format!("{sign}{oh:02}{om:02}"),
                _ => format!("{sign}{oh:02}:{om:02}"),
            }
        }
        other => std::iter::repeat_n(other, n).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2019-03-13 15:11:48 UTC, a Wednesday.
    const AT: i64 = 1_552_489_908_000;

    #[test]
    fn presets_and_patterns() {
        assert_eq!(format("datetime", "UTC", AT, 0), "2019-03-13 15:11:48");
        assert_eq!(format("time2", "+01:00", AT, 0), "16:11:48 GMT+1 GMT+0100");
        assert_eq!(
            format("eeee, dd. MMMM yyyy", "", AT, 0),
            "Wednesday, 13. March 2019"
        );
        assert_eq!(
            format("h:mm a 'o''clock' xxx", "UTC-5", AT, 0),
            "10:11 AM o'clock -05:00"
        );
        assert_eq!(format("yy/M/d", "Somewhere", AT, 3600), "19/3/13");
        assert_eq!(format("HH", "Somewhere", AT, 3600), "16");
    }
}
