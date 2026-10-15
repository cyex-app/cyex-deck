//! V105-16 Constant bodies for sets PCRE2 spells and Rust doesn't.

use super::Ascii;

/// PCRE2's `\h` (also `[:blank:]`).
pub(super) const H: &str = r"\t\x{20}\x{A0}\x{1680}\x{180E}\x{2000}-\x{200A}\x{202F}\x{205F}\x{3000}";
/// PCRE2's `\v`.
pub(super) const V: &str = r"\n\x0B\f\r\x{85}\x{2028}\x{2029}";

const SPACE: &str = r"\p{Z}\t\n\x0B\f\r";
const GRAPH: &str = r"\p{L}\p{M}\p{N}\p{P}\p{S}\p{Co}[\p{Cf}--[\x{61C}\x{180E}\x{2066}-\x{2069}]]";

/// `[:name:]` under PCRE2_UCP (or ASCII under `(?aP)`/`(?aT)`): the body for inside a class.
pub(super) fn posix(name: &str, ascii: Ascii) -> Option<String> {
    const NAMES: [&str; 14] = [
        "alnum", "alpha", "blank", "cntrl", "digit", "lower", "upper", "space", "word", "xdigit", "ascii",
        "punct", "graph", "print",
    ];
    if !NAMES.contains(&name) {
        return None;
    }
    // Rust's own classes are ASCII.
    if ascii.posix || (ascii.digit && name == "digit") {
        return Some(format!("[[:{name}:]]"));
    }
    Some(match name {
        "alnum" => r"\p{L}\p{N}".to_owned(),
        "alpha" => r"\p{L}".to_owned(),
        "blank" => H.to_owned(),
        "cntrl" => r"\p{Cc}".to_owned(),
        "digit" => r"\p{Nd}".to_owned(),
        "lower" => r"\p{Ll}".to_owned(),
        "upper" => r"\p{Lu}".to_owned(),
        "space" => SPACE.to_owned(),
        "word" => r"\w".to_owned(),
        "xdigit" => "0-9A-Fa-f".to_owned(),
        "ascii" => r"\x00-\x7F".to_owned(),
        "punct" => r"\p{P}[\p{S}&&\x00-\xFF]".to_owned(),
        "graph" => GRAPH.to_owned(),
        _ => format!(r"{GRAPH}\p{{Zs}}"),
    })
}

/// PCRE2-only `\p{…}` names (Xan, Xps, Xsp, Xuc, Xwd): the body for inside a class.
pub(super) fn property(name: &str) -> Option<&'static str> {
    Some(match name {
        "Xan" => r"\p{L}\p{N}",
        "Xps" | "Xsp" => SPACE,
        "Xwd" => r"\w",
        "Xuc" => r"$@\x{60}\x{A0}-\x{D7FF}\x{E000}-\x{10FFFF}",
        _ => return None,
    })
}
