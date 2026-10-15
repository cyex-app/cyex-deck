//! V1051-04 The 7TV variables: `{paint}`, `{paint.name}`, `{paint.id}`, `{badge}`, `{badge.name}`,
//! `{badge.id}`, `{combo}`, `{combo:<name>}`. Anything else in braces is text.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Var {
    Paint,
    PaintName,
    PaintId,
    Badge,
    BadgeName,
    BadgeId,
    Combo,
    ComboApply(String),
}

/// How each is written, for the registry (group 5) and its check.
pub const FORMS: [&str; 8] = [
    "{paint}",
    "{paint.name}",
    "{paint.id}",
    "{badge}",
    "{badge.name}",
    "{badge.id}",
    "{combo}",
    "{combo:<name>}",
];

const NAME_MAX: usize = 64;

/// `chars[at]` is `{`. The variable and the index after its `}`, or `None` (then it's text).
pub(super) fn read(chars: &[char], at: usize) -> Option<(Var, usize)> {
    let rest = chars.get(at + 1..)?;
    let close = rest.iter().position(|&c| c == '}')?;
    let inner = &rest[..close];
    let next = at + 1 + close + 1;
    let word: String = inner.iter().collect();
    let lower = word.to_ascii_lowercase();
    let var = match lower.as_str() {
        "paint" => Var::Paint,
        "paint.name" => Var::PaintName,
        "paint.id" => Var::PaintId,
        "badge" => Var::Badge,
        "badge.name" => Var::BadgeName,
        "badge.id" => Var::BadgeId,
        "combo" => Var::Combo,
        _ => {
            let (key, name) = word.split_once(':')?;
            if !key.eq_ignore_ascii_case("combo")
                || name.chars().count() > NAME_MAX
                || name.contains(['{', '\n', '\r'])
            {
                return None;
            }
            Var::ComboApply(name.trim().to_owned())
        }
    };
    Some((var, next))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> Option<Var> {
        let chars: Vec<char> = s.chars().collect();
        read(&chars, 0).map(|(v, n)| {
            assert_eq!(n, chars.len());
            v
        })
    }

    #[test]
    fn all_forms() {
        assert_eq!(r("{paint}"), Some(Var::Paint));
        assert_eq!(r("{paint.name}"), Some(Var::PaintName));
        assert_eq!(r("{paint.id}"), Some(Var::PaintId));
        assert_eq!(r("{badge}"), Some(Var::Badge));
        assert_eq!(r("{badge.name}"), Some(Var::BadgeName));
        assert_eq!(r("{badge.id}"), Some(Var::BadgeId));
        assert_eq!(r("{combo}"), Some(Var::Combo));
        assert_eq!(r("{combo:x}"), Some(Var::ComboApply("x".into())));
        assert_eq!(r("{PAINT}"), Some(Var::Paint));
        assert_eq!(
            r("{combo: night owl }"),
            Some(Var::ComboApply("night owl".into()))
        );
        assert_eq!(r("{combo:}"), Some(Var::ComboApply(String::new())));
    }

    #[test]
    fn not_variables() {
        for s in ["{paint", "{paint.foo}", "{ paint}", "{\"a\":1}", "{combo:a{b}"] {
            assert_eq!(r(s), None, "{s}");
        }
        let long = format!("{{combo:{}}}", "a".repeat(65));
        assert_eq!(r(&long), None);
        let ok = format!("{{combo:{}}}", "a".repeat(64));
        assert!(r(&ok).is_some());
    }

    #[test]
    fn forms_read_back() {
        for f in FORMS {
            let f = f.replace("<name>", "n");
            assert!(r(&f).is_some(), "{f}");
        }
    }
}
