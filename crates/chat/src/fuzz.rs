//! V110-F89 Test-only generator for fuzz and property tests (no fuzzing crate is available).
//! Deterministic: every test seeds it with a constant so a failure reproduces. Other crates
//! include this file with `#[path = "../../chat/src/fuzz.rs"]`.
#![allow(dead_code)]

pub struct Rng(pub u64);

/// Characters that tend to break parsers: case mappings that change length, combining and
/// bidi marks, zero-width characters, astral emoji, controls, delimiters.
const TROUBLE: &[char] = &[
    '\u{130}', '\u{212A}', '\u{1E9E}', 'ß', 'ǅ', 'ŉ', 'ﬃ', 'İ', 'ı', 'ς', 'Σ', '\u{2126}',
    '\u{300}', '\u{301}', '\u{34F}', '\u{200B}', '\u{200C}', '\u{200D}', '\u{2060}', '\u{FEFF}',
    '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2066}',
    '\u{2067}', '\u{2068}', '\u{2069}', '\u{FFFD}', '\u{FFFF}', '\u{10FFFF}', '\u{E000}', '\u{D7FF}',
    '😀', '👋', '\u{1F3FD}', '👨', '👩', '\u{FE0F}', '\u{20E3}', '🇩', '🇪', '𝕏', '𠮷', '\u{E0001}',
    '\0', '\u{1}', '\u{7}', '\u{8}', '\t', '\u{B}', '\u{C}', '\r', '\n', '\u{1B}', '\u{7F}', '\u{80}',
    '\u{85}', '\u{9F}', '\u{A0}', '\u{2028}', '\u{2029}', '\u{3000}', ' ', ' ', ' ',
    '\\', ';', ':', '#', '@', '!', '$', '(', ')', '{', '}', '[', ']', '|', '/', '"', '\'', '%', '&',
    '=', ',', '.', '-', '+', '*', '?', '^', '<', '>', '~', '`',
];

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    pub fn scalar(&mut self) -> char {
        match self.below(10) {
            0..=4 => *self.pick(TROUBLE),
            5..=6 => char::from(b'a' + self.below(26) as u8),
            7 => char::from(b'0' + self.below(10) as u8),
            8 => char::from_u32(self.below(0x3000) as u32 + 0x80).unwrap_or('?'),
            _ => loop {
                if let Some(c) = char::from_u32(self.next() as u32 % 0x11_0000) {
                    break c;
                }
            },
        }
    }

    /// Up to `max` scalars.
    pub fn text(&mut self, max: usize) -> String {
        let n = self.below(max + 1);
        (0..n).map(|_| self.scalar()).collect()
    }

    /// Words joined by spaces, a bit like chat.
    pub fn words(&mut self, max_words: usize) -> String {
        let n = self.below(max_words + 1);
        let mut out = String::new();
        for i in 0..n {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(&self.text(8));
        }
        out
    }

    /// Random bytes, decoded lossily.
    pub fn lossy(&mut self, max: usize) -> String {
        let n = self.below(max + 1);
        let bytes: Vec<u8> = (0..n).map(|_| self.next() as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    pub fn number_text(&mut self) -> String {
        const N: &[&str] = &[
            "0", "-1", "1", "-0", "255", "256", "65535", "2147483647", "2147483648", "4294967295",
            "4294967296", "9223372036854775807", "9223372036854775808", "18446744073709551615",
            "18446744073709551616", "99999999999999999999999999", "-9223372036854775808",
            "1e308", "1e309", "-1e309", "NaN", "Infinity", "0.1", "1.5e-320", "0x10", "1_000", "٣",
        ];
        self.pick(N).to_string()
    }

    /// Structure-aware mutation of a valid sample.
    pub fn mutate(&mut self, s: &str) -> String {
        let mut out: Vec<char> = s.chars().collect();
        for _ in 0..1 + self.below(3) {
            match self.below(9) {
                // Truncate at any byte offset (lossy when it cuts a character).
                0 => {
                    let bytes = out.iter().collect::<String>().into_bytes();
                    let cut = self.below(bytes.len() + 1);
                    out = String::from_utf8_lossy(&bytes[..cut]).chars().collect();
                }
                // Duplicate a span.
                1 if !out.is_empty() => {
                    let a = self.below(out.len());
                    let b = (a + self.below(out.len() - a + 1)).min(out.len());
                    let span: Vec<char> = out[a..b].to_vec();
                    let at = self.below(out.len() + 1);
                    for (i, c) in span.into_iter().enumerate() {
                        out.insert(at + i, c);
                    }
                }
                // Delete a span.
                2 if !out.is_empty() => {
                    let a = self.below(out.len());
                    let b = (a + self.below(out.len() - a + 1)).min(out.len());
                    out.drain(a..b);
                }
                // Swap a delimiter.
                3 if !out.is_empty() => {
                    let delims: Vec<usize> = out
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| !c.is_alphanumeric())
                        .map(|(i, _)| i)
                        .collect();
                    if !delims.is_empty() {
                        let i = delims[self.below(delims.len())];
                        out[i] = *self.pick(&[',', ';', ':', '/', '-', '|', ' ', '=', '"', '\\', '&', '(', ')']);
                    }
                }
                // A huge or odd number.
                4 => {
                    let n = self.number_text();
                    let at = self.below(out.len() + 1);
                    for (i, c) in n.chars().enumerate() {
                        out.insert(at + i, c);
                    }
                }
                // A very long field.
                5 => {
                    let at = self.below(out.len() + 1);
                    let c = self.scalar();
                    let n = 1 + self.below(2_000);
                    out.splice(at..at, std::iter::repeat_n(c, n));
                }
                // Random scalars.
                6 => {
                    let at = self.below(out.len() + 1);
                    for _ in 0..1 + self.below(4) {
                        let c = self.scalar();
                        out.insert(at, c);
                    }
                }
                // Replace one scalar.
                7 if !out.is_empty() => {
                    let i = self.below(out.len());
                    out[i] = self.scalar();
                }
                // Swap two scalars.
                _ if out.len() > 1 => {
                    let (a, b) = (self.below(out.len()), self.below(out.len()));
                    out.swap(a, b);
                }
                _ => {}
            }
        }
        out.into_iter().collect()
    }
}

/// Every prefix of `s`, cut at every byte offset (a cut inside a character is decoded
/// lossily).
pub fn byte_prefixes(s: &str) -> Vec<String> {
    let b = s.as_bytes();
    (0..=b.len()).map(|i| String::from_utf8_lossy(&b[..i]).into_owned()).collect()
}

/// Every prefix of `s` on character boundaries.
pub fn char_prefixes(s: &str) -> Vec<&str> {
    s.char_indices().map(|(i, _)| &s[..i]).chain([s]).collect()
}
