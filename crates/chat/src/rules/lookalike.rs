//! C32 Letters that look like Latin ones count as them when rules match: Cyrillic and
//! Greek look-alikes, fullwidth letters and a few accented forms (Chatty's idea).

/// Folds look-alikes to plain ASCII. Leaves everything else alone.
pub fn fold(text: &str) -> String {
    text.chars().map(fold_char).collect()
}

fn fold_char(c: char) -> char {
    // Fullwidth ASCII: ！ to ～.
    if ('\u{FF01}'..='\u{FF5E}').contains(&c) {
        return char::from_u32(c as u32 - 0xFEE0).unwrap_or(c);
    }
    match c {
        'а' => 'a',
        'А' | 'Α' => 'A',
        'в' | 'В' | 'Β' => 'B',
        'с' | 'ϲ' => 'c',
        'С' | 'Ϲ' => 'C',
        'е' | 'ё' => 'e',
        'Е' | 'Ё' | 'Ε' => 'E',
        'һ' => 'h',
        'Н' | 'Η' => 'H',
        'і' | 'ι' => 'i',
        'І' | 'Ι' | 'Ӏ' => 'I',
        'ј' => 'j',
        'Ј' => 'J',
        'к' | 'κ' => 'k',
        'К' | 'Κ' => 'K',
        'М' | 'Μ' => 'M',
        'п' => 'n',
        'Ν' => 'N',
        'о' | 'ο' | 'σ' => 'o',
        'О' | 'Ο' => 'O',
        'р' | 'ρ' => 'p',
        'Р' | 'Ρ' => 'P',
        'ԛ' => 'q',
        'ѕ' => 's',
        'Ѕ' => 'S',
        'т' | 'τ' => 't',
        'Т' | 'Τ' => 'T',
        'υ' | 'ս' => 'u',
        'ν' | 'ѵ' => 'v',
        'ш' | 'ѡ' => 'w',
        'х' | 'χ' => 'x',
        'Х' | 'Χ' => 'X',
        'у' | 'γ' => 'y',
        'У' | 'Υ' | 'Ү' => 'Y',
        'Ζ' => 'Z',
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ı' => 'i',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds() {
        assert_eq!(fold("рау раl"), "pay pal");
        assert_eq!(fold("ＦＲＥＥ"), "FREE");
        assert_eq!(fold("Gräfin 日本"), "Grafin 日本");
    }
}
