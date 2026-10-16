//! V1051-04 Every chat-side sentence of the 7TV commands, in one place.

use super::apply::Part;
use super::{SelectError, Why};

pub fn select_error(e: &SelectError) -> String {
    match e {
        SelectError::NoConsent => "Allow 7TV first: /cosmetics.".into(),
        SelectError::SignedOut => "Sign in to 7TV first: /cosmetics.".into(),
        SelectError::SignInAgain => "Your 7TV sign-in ran out. /cosmetics signs you in again.".into(),
        SelectError::NotAllowed => "7TV didn’t allow that one.".into(),
        SelectError::Down => "Couldn’t reach 7TV. Nothing changed.".into(),
        SelectError::RateLimited { seconds } => format!("7TV needs a short break. Try again in {}.", wait(*seconds)),
    }
}

/// "30 s" under a minute, else whole minutes rounded up ("60 min" at most: 7TV's cap is an hour).
fn wait(seconds: u64) -> String {
    if seconds < 60 { format!("{seconds} s") } else { format!("{} min", seconds.div_ceil(60)) }
}

pub fn why(w: &Why) -> &'static str {
    match w {
        Why::NoConsent => "Allow 7TV first: /cosmetics.",
        Why::NoAccount => "This account has no 7TV account yet. Sign in once on 7tv.app.",
        Why::Down => "Couldn’t reach 7TV. Try again in a minute.",
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_lowercase().chain(c).collect())
        .unwrap_or_default()
}

/// One part's error inside a longer line: "Nothing changed." would be wrong there.
fn part_error(e: &SelectError) -> String {
    match e {
        SelectError::Down => "couldn’t reach 7TV.".into(),
        e => lower_first(&select_error(e)),
    }
}

fn reason(p: &Part, plural: bool) -> Option<&'static str> {
    match (p, plural) {
        (Part::NotOwned, false) => Some("it’s not yours any more"),
        (Part::NotOwned, true) => Some("they’re not yours any more"),
        (Part::NeedsSub, false) => Some("it’s for 7TV subscribers"),
        (Part::NeedsSub, true) => Some("they’re for 7TV subscribers"),
        _ => None,
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// What to tell the user after a pair went on. `name` is the combo’s; none for Undo.
pub fn applied(name: Option<&str>, paint: &Part, badge: &Part) -> String {
    let parts = [("paint", paint), ("badge", badge)];
    let skips: Vec<_> = parts.iter().filter(|p| reason(p.1, false).is_some()).collect();
    let fails: Vec<_> = parts
        .iter()
        .filter_map(|(k, p)| match p {
            Part::Failed { error } => Some((*k, error)),
            _ => None,
        })
        .collect();
    let mut out: Vec<String> = Vec::new();
    if skips.len() == 2 {
        // Same reason: one plural sentence. Different: both, joined.
        let r = if skips[0].1 == skips[1].1 {
            reason(skips[0].1, true).unwrap_or_default().to_owned()
        } else {
            format!(
                "{} and {}",
                reason(skips[0].1, false).unwrap_or_default(),
                reason(skips[1].1, false).unwrap_or_default()
            )
        };
        out.push(format!("Nothing changed: {r}."));
    } else if let [(k, p)] = skips.as_slice() {
        let r = reason(p, false).unwrap_or_default();
        out.push(match name {
            // The other part failed too: nothing is worn as asked.
            _ if !fails.is_empty() => format!("The {k} didn’t change: {r}."),
            Some(n) => format!("Wearing {n} without its {k}: {r}."),
            None => format!("Done. Not the {k}: {r}."),
        });
    } else if fails.is_empty() {
        out.push(match name {
            Some(n) => format!("Wearing {n}"),
            None => "Done.".into(),
        });
    }
    for (k, e) in &fails {
        let other = if *k == "paint" { badge } else { paint };
        let other_name = if *k == "paint" { "badge" } else { "paint" };
        let err = part_error(e);
        out.push(if *other == Part::Changed {
            format!("{} changed. The {k} didn’t: {err}", capital(other_name))
        } else {
            format!("The {k} didn’t change: {err}")
        });
    }
    out.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fail(e: SelectError) -> Part {
        Part::Failed { error: e }
    }

    #[test]
    fn named_all_good() {
        assert_eq!(
            applied(Some("Night"), &Part::Changed, &Part::Same),
            "Wearing Night"
        );
    }

    #[test]
    fn named_one_skipped() {
        assert_eq!(
            applied(Some("Night"), &Part::Changed, &Part::NotOwned),
            "Wearing Night without its badge: it’s not yours any more."
        );
        assert_eq!(
            applied(Some("Night"), &Part::NeedsSub, &Part::Same),
            "Wearing Night without its paint: it’s for 7TV subscribers."
        );
    }

    #[test]
    fn named_both_skipped() {
        assert_eq!(
            applied(Some("N"), &Part::NotOwned, &Part::NotOwned),
            "Nothing changed: they’re not yours any more."
        );
        assert_eq!(
            applied(Some("N"), &Part::NeedsSub, &Part::NeedsSub),
            "Nothing changed: they’re for 7TV subscribers."
        );
        assert_eq!(
            applied(Some("N"), &Part::NeedsSub, &Part::NotOwned),
            "Nothing changed: it’s for 7TV subscribers and it’s not yours any more."
        );
    }

    #[test]
    fn named_one_failed() {
        assert_eq!(
            applied(Some("N"), &Part::Changed, &fail(SelectError::Down)),
            "Paint changed. The badge didn’t: couldn’t reach 7TV."
        );
        assert_eq!(
            applied(Some("N"), &Part::NotOwned, &fail(SelectError::Down)),
            "The paint didn’t change: it’s not yours any more. The badge didn’t change: couldn’t reach 7TV."
        );
        assert_eq!(
            applied(None, &fail(SelectError::NotAllowed), &Part::Changed),
            "Badge changed. The paint didn’t: 7TV didn’t allow that one."
        );
        assert_eq!(
            applied(None, &Part::Same, &fail(SelectError::RateLimited { seconds: 4 })),
            "The badge didn’t change: 7TV needs a short break. Try again in 4 s."
        );
    }

    #[test]
    fn unnamed() {
        assert_eq!(applied(None, &Part::Changed, &Part::Overtaken), "Done.");
        assert_eq!(
            applied(None, &Part::Changed, &Part::NotOwned),
            "Done. Not the badge: it’s not yours any more."
        );
    }

    #[test]
    fn long_waits_are_minutes() {
        let limited = |seconds| select_error(&SelectError::RateLimited { seconds });
        assert_eq!(limited(59), "7TV needs a short break. Try again in 59 s.");
        assert_eq!(limited(61), "7TV needs a short break. Try again in 2 min.");
        assert_eq!(limited(3600), "7TV needs a short break. Try again in 60 min.");
    }

    #[test]
    fn error_sentences() {
        assert_eq!(
            select_error(&SelectError::NoConsent),
            "Allow 7TV first: /cosmetics."
        );
        assert_eq!(
            select_error(&SelectError::SignedOut),
            "Sign in to 7TV first: /cosmetics."
        );
        assert_eq!(
            why(&Why::NoAccount),
            "This account has no 7TV account yet. Sign in once on 7tv.app."
        );
    }
}
