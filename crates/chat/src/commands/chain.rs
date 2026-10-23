//! S3 Chatty's `/chain`, `/foreach` and `/timer` arguments (`/runin` needs nothing).

/// `/chain a | b`: the parts, trimmed; `||` is a literal `|`.
pub fn split_chain(text: &str) -> Vec<String> {
    split_on(text, '|')
        .into_iter()
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
        .collect()
}

/// `/foreach a b c > /cmd $1`: the items and the command. The first single `>` splits;
/// `>>` is a literal `>`.
pub fn split_foreach(text: &str) -> Option<(Vec<String>, String)> {
    let parts = split_on(text, '>');
    let mut parts = parts.into_iter();
    let list = parts.next()?;
    let rest: Vec<String> = parts.collect();
    if rest.is_empty() {
        return None;
    }
    let command = rest.join(">").trim().to_owned();
    let items: Vec<String> = list.split_whitespace().map(str::to_owned).collect();
    (!items.is_empty() && !command.is_empty()).then_some((items, command))
}

/// Splits on single `c`; a run of n copies stands for n-1 literal ones.
fn split_on(text: &str, c: char) -> Vec<String> {
    let mut out = vec![String::new()];
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == c {
            let mut run = 0;
            while i < chars.len() && chars[i] == c {
                run += 1;
                i += 1;
            }
            if run == 1 {
                out.push(String::new());
            } else {
                out.last_mut()
                    .expect("never empty")
                    .extend(std::iter::repeat_n(c, run - 1));
            }
        } else {
            out.last_mut().expect("never empty").push(chars[i]);
            i += 1;
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Timer {
    List,
    /// An id, or `*` for all.
    Stop(String),
    Start(TimerStart),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimerStart {
    pub id: Option<String>,
    /// `-r`: kept through a restart.
    pub keep: bool,
    /// `-s`: no "Timer started" line.
    pub silent: bool,
    /// `-o`: replaces a timer with the same id.
    pub overwrite: bool,
    pub when: When,
    pub command: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    /// From now.
    In { secs: u64 },
    /// The next time the clock shows this, seconds after midnight.
    At { day_secs: u32 },
    /// A date and time, local: days since 1970 plus seconds after midnight.
    On { days: i64, day_secs: u32 },
}

/// `/timer [-rso] [:id] <5m|17:30|[2026/10/11 17:30]|list|stop id> <command>`. The date takes
/// `/` (3.1) or Chatty's `-`.
pub fn parse_timer(args: &str) -> Result<Timer, String> {
    let mut rest = args.trim();
    let mut start = TimerStart {
        id: None,
        keep: false,
        silent: false,
        overwrite: false,
        when: When::In { secs: 0 },
        command: String::new(),
    };
    if let Some(opts) = rest.strip_prefix('-') {
        let (o, r) = opts.split_once(' ').unwrap_or((opts, ""));
        for c in o.chars() {
            match c {
                'r' => start.keep = true,
                's' => start.silent = true,
                'o' => start.overwrite = true,
                'a' | 'q' => {}
                _ => return Err(format!("Unknown timer option -{c}.")),
            }
        }
        rest = r.trim_start();
    }
    if let Some(idrest) = rest.strip_prefix(':') {
        let (id, r) = idrest.split_once(' ').unwrap_or((idrest, ""));
        start.id = Some(id.to_owned());
        rest = r.trim_start();
    }
    let (head, tail) = if rest.starts_with('[') {
        let end = rest.find(']').ok_or("A ‘]’ is missing.")?;
        (&rest[..=end], rest[end + 1..].trim_start())
    } else {
        rest.split_once(' ')
            .map_or((rest, ""), |(h, t)| (h, t.trim_start()))
    };
    match head {
        "list" => return Ok(Timer::List),
        "stop" => {
            let id = tail.trim();
            let id = if id.is_empty() {
                start.id.ok_or("Say which timer: /timer stop <id>")?
            } else {
                id.to_owned()
            };
            return Ok(Timer::Stop(id));
        }
        _ => {}
    }
    if tail.is_empty() {
        return Err("Usage: /timer 5m <command>".into());
    }
    start.when = when(head).ok_or_else(|| format!("Can’t read the time ‘{head}’."))?;
    start.command = tail.to_owned();
    Ok(Timer::Start(start))
}

fn when(text: &str) -> Option<When> {
    if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        let inner = inner.trim();
        return match inner.split_once(' ') {
            Some((date, time)) => {
                let mut d = date.split(['/', '-']).map(|p| p.parse::<i64>().ok());
                let (y, m, dd) = (d.next()??, d.next()??, d.next()??);
                Some(When::On {
                    days: days_from_civil(y, m, dd)?,
                    day_secs: clock(time)?,
                })
            }
            None => Some(When::At {
                day_secs: clock(inner)?,
            }),
        };
    }
    if text.contains(':') {
        return Some(When::At {
            day_secs: clock(text)?,
        });
    }
    duration(text).map(|secs| When::In { secs })
}

/// `17:30` or `5:30:50`.
fn clock(text: &str) -> Option<u32> {
    let parts: Vec<u32> = text.split(':').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let (h, m, s) = match parts[..] {
        [h, m] => (h, m, 0),
        [h, m, s] => (h, m, s),
        _ => return None,
    };
    (h < 24 && m < 60 && s < 60).then_some(h * 3600 + m * 60 + s)
}

/// `90`, `5m`, `1h30m`, `5h10` (= 5h10m), `500ms`.
fn duration(text: &str) -> Option<u64> {
    if let Ok(n) = text.parse::<u64>() {
        return Some(n);
    }
    let mut total_ms: u64 = 0;
    let mut num = String::new();
    let mut last_unit = 's';
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            num.push(c);
            i += 1;
            continue;
        }
        let n: u64 = num.parse().ok()?;
        num.clear();
        let (unit_ms, unit) = if c == 'm' && chars.get(i + 1) == Some(&'s') {
            i += 1;
            (1, 'x')
        } else {
            match c {
                's' => (1000, 's'),
                'm' => (60_000, 'm'),
                'h' => (3_600_000, 'h'),
                'd' => (86_400_000, 'd'),
                _ => return None,
            }
        };
        total_ms += n * unit_ms;
        last_unit = unit;
        i += 1;
    }
    if !num.is_empty() {
        // A bare number after a unit is the next smaller unit (`5h10` = 5h10m).
        let n: u64 = num.parse().ok()?;
        total_ms += n * match last_unit {
            'd' => 3_600_000,
            'h' => 60_000,
            _ => 1000,
        };
    }
    (total_ms > 0).then_some(total_ms.div_ceil(1000))
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chains_and_loops() {
        assert_eq!(split_chain("/echo a | /echo b||c |"), ["/echo a", "/echo b|c"]);
        let (items, cmd) = split_foreach("a b  c > /echo $1 >> x").unwrap();
        assert_eq!(items, ["a", "b", "c"]);
        assert_eq!(cmd, "/echo $1 > x");
        assert!(split_foreach("a b c").is_none());
    }

    #[test]
    fn timers() {
        let Timer::Start(t) = parse_timer("-rs :abc 5h10 /echo hi").unwrap() else {
            panic!()
        };
        assert!(t.keep && t.silent && !t.overwrite);
        assert_eq!(
            (t.id.as_deref(), t.when, t.command.as_str()),
            (Some("abc"), When::In { secs: 18_600 }, "/echo hi")
        );
        let Timer::Start(t) = parse_timer("17:30 /echo x").unwrap() else {
            panic!()
        };
        assert_eq!(t.when, When::At { day_secs: 63_000 });
        let Timer::Start(t) = parse_timer("[2022-01-10 5:30] /echo x").unwrap() else {
            panic!()
        };
        assert_eq!(
            t.when,
            When::On {
                days: 19_002,
                day_secs: 19_800
            }
        );
        let Timer::Start(t) = parse_timer("[2022/01/10 5:30] /echo x").unwrap() else {
            panic!()
        };
        assert_eq!(t.when, When::On { days: 19_002, day_secs: 19_800 });
        let Timer::Start(t) = parse_timer("500ms x").unwrap() else {
            panic!()
        };
        assert_eq!(t.when, When::In { secs: 1 });
        assert_eq!(parse_timer("list").unwrap(), Timer::List);
        assert_eq!(parse_timer("stop *").unwrap(), Timer::Stop("*".into()));
        assert!(parse_timer("5m").is_err());
        assert!(parse_timer("-z 5m x").is_err());
        assert!(parse_timer("25:00 x").is_err());
    }
}
