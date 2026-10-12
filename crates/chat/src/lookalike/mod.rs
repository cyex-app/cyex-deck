//! C32, V104-01 Letters that look like ASCII ones count as them when rules and mentions
//! match: Cyrillic, Greek, fullwidth, mathematical letters and the rest of Unicode's
//! confusables that look like one letter or digit, plus Latin letters with accents. The
//! table is generated (`tools/confusables.mjs`); ASCII itself is never changed, so plain
//! ASCII text needs no folding at all.

mod table;

use table::RUNS;

/// Folds look-alikes to plain ASCII. Leaves everything else alone.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    fold_into(text, &mut out);
    out
}

/// Folds into `out` (cleared first), so a caller can reuse one buffer.
pub fn fold_into(text: &str, out: &mut String) {
    out.clear();
    if text.is_ascii() {
        out.push_str(text);
        return;
    }
    out.extend(text.chars().map(fold_char));
}

/// The ASCII letter or digit `c` looks like, or `c`.
pub fn fold_char(c: char) -> char {
    let cp = c as u32;
    if cp < 0x80 {
        return c;
    }
    let i = RUNS.partition_point(|r| r.0 <= cp);
    let Some(&(start, len, first, step)) = i.checked_sub(1).map(|i| &RUNS[i]) else {
        return c;
    };
    let offset = cp - start;
    if offset >= u32::from(len) {
        return c;
    }
    char::from(first + step * offset as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds() {
        assert_eq!(fold("рау раl"), "pay pal", "Cyrillic");
        assert_eq!(fold("ＦＲＥＥ ２"), "FREE 2", "fullwidth");
        assert_eq!(
            fold("𝐟𝐚𝐧𝐜𝐲 𝟎"),
            "fancy 0",
            "mathematical bold, digits stay digits"
        );
        assert_eq!(fold("Gräfin 日本"), "Grafin 日本", "accents go, the rest stays");
        assert_eq!(fold("ΙРL"), "IPL", "capital iota is an I, not an l");
        assert_eq!(fold("plain ascii 123"), "plain ascii 123");
    }

    #[test]
    fn table_is_sorted_and_ascii() {
        for w in RUNS.windows(2) {
            assert!(w[0].0 + u32::from(w[0].1) <= w[1].0, "{:x}", w[1].0);
        }
        for &(start, len, first, step) in RUNS {
            assert!(start >= 0x80 && len > 0 && step <= 1);
            assert!((first + step * (len - 1)).is_ascii_alphanumeric(), "{start:x}");
        }
    }
}
