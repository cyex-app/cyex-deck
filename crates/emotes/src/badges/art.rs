//! V108-C10 Same badge art under different URLs. A picture's print is its first frame scaled to
//! 8 x 8 (colour times alpha, RGBA); two prints a few levels apart on average are the same art
//! (a re-upload re-encodes it, so bytes never match). Only pictures that meet on one chat line
//! are fetched (`Cosmetics` asks for them); prints are kept on disk by URL so a restart fetches
//! nothing it already has. The first URL with a print stays the canonical one.

use std::collections::HashMap;

/// Side of a print, in pixels.
pub const SIDE: u32 = 8;
const LEN: usize = (SIDE * SIDE * 4) as usize;
/// Twice the mean difference per channel (0..255) still counted as the same art: 2.5. Re-encodes
/// of one badge measured 0 to 2.5; the closest different badges 3.7 and up (Homies, DankChat,
/// FFZ:AP, BadgeHQ, Chatsen, 1.026 pictures, 2026/10/30).
const ALIKE_TWICE: u32 = 5;
/// Mean alpha below this: too little picture to compare (a dot on a clear square), never merged.
const MIN_COVER: u32 = 32;
/// Prints kept, on disk and in memory; past it the oldest go.
pub const MAX: usize = 2_000;

/// An 8 x 8 picture, RGBA with the colour already times alpha.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Print(Vec<u8>);

impl Print {
    /// From the 8 x 8 BGRA pixels Windows hands back; None for the wrong size or a nearly
    /// clear picture.
    pub fn from_bgra(px: &[u8]) -> Option<Print> {
        if px.len() != LEN {
            return None;
        }
        let mut out = Vec::with_capacity(LEN);
        for &[b, g, r, a] in px.as_chunks::<4>().0 {
            let mul = |c: u8| (u16::from(c) * u16::from(a) / 255) as u8;
            out.extend([mul(r), mul(g), mul(b), a]);
        }
        let cover: u32 = out.iter().skip(3).step_by(4).map(|&a| u32::from(a)).sum();
        (cover >= MIN_COVER * SIDE * SIDE).then_some(Print(out))
    }

    pub fn alike(&self, other: &Print) -> bool {
        let diff: u32 = self.0.iter().zip(&other.0).map(|(a, b)| u32::from(a.abs_diff(*b))).sum();
        diff * 2 <= ALIKE_TWICE * LEN as u32
    }

    fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn from_hex(s: &str) -> Option<Print> {
        if s.len() != LEN * 2 || !s.is_ascii() {
            return None;
        }
        (0..LEN)
            .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
            .collect::<Option<Vec<_>>>()
            .map(Print)
    }
}

/// One picture: its print (None: wouldn't decode or nearly clear) and the URL it counts as.
#[derive(Clone, Debug, PartialEq)]
struct Entry {
    url: String,
    print: Option<Print>,
    canon: String,
}

/// Prints in the order they came, with each picture's canonical URL.
#[derive(Debug, Default, PartialEq)]
pub struct Prints {
    list: Vec<Entry>,
    index: HashMap<String, usize>,
}

impl Prints {
    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn has(&self, url: &str) -> bool {
        self.index.contains_key(url)
    }

    pub fn urls(&self) -> impl Iterator<Item = &str> {
        self.list.iter().map(|e| e.url.as_str())
    }

    /// Adds a picture's print; it counts as the first earlier picture with the same art, else
    /// as itself. True when that made it count as another URL. A URL already in keeps its place.
    pub fn add(&mut self, url: &str, print: Option<Print>) -> bool {
        if self.has(url) {
            return false;
        }
        let canon = print
            .as_ref()
            .and_then(|p| self.list.iter().find(|e| e.print.as_ref().is_some_and(|q| q.alike(p))))
            .map_or_else(|| url.to_owned(), |e| e.canon.clone());
        let merged = canon != url;
        self.index.insert(url.to_owned(), self.list.len());
        self.list.push(Entry {
            url: url.to_owned(),
            print,
            canon,
        });
        self.trim();
        merged
    }

    /// Past `MAX` the oldest go. A canonical URL that goes still names the art for the rest.
    fn trim(&mut self) {
        if self.list.len() <= MAX {
            return;
        }
        let drop = self.list.len() - MAX;
        self.list.drain(..drop);
        self.index = self.list.iter().enumerate().map(|(i, e)| (e.url.clone(), i)).collect();
    }

    /// URL → canonical URL, only where they differ (what `Cosmetics::set_art` takes).
    pub fn map(&self) -> HashMap<String, String> {
        self.list
            .iter()
            .filter(|e| e.canon != e.url)
            .map(|e| (e.url.clone(), e.canon.clone()))
            .collect()
    }

    /// The file: `[[url, print hex or "", canon], …]`, oldest first.
    pub fn to_json(&self) -> String {
        let rows: Vec<[String; 3]> = self
            .list
            .iter()
            .map(|e| [e.url.clone(), e.print.as_ref().map(Print::hex).unwrap_or_default(), e.canon.clone()])
            .collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
    }

    /// A broken file reads as empty; bad rows are skipped.
    pub fn from_json(text: &str) -> Prints {
        let rows: Vec<[String; 3]> = serde_json::from_str(text).unwrap_or_default();
        let mut out = Prints::default();
        for [url, hex, canon] in rows {
            if url.is_empty() || canon.is_empty() || out.has(&url) {
                continue;
            }
            let print = if hex.is_empty() { None } else { Print::from_hex(&hex) };
            if print.is_none() && !hex.is_empty() {
                continue;
            }
            out.index.insert(url.clone(), out.list.len());
            out.list.push(Entry { url, print, canon });
        }
        out.trim();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A solid 8 x 8 BGRA picture.
    fn solid(b: u8, g: u8, r: u8, a: u8) -> Vec<u8> {
        [b, g, r, a].repeat((SIDE * SIDE) as usize)
    }

    fn print(b: u8, g: u8, r: u8) -> Option<Print> {
        Print::from_bgra(&solid(b, g, r, 255))
    }

    #[test]
    fn prints_compare_with_a_little_slack() {
        let red = print(0, 0, 200).unwrap();
        assert!(red.alike(&print(1, 1, 201).unwrap()), "a re-encode");
        assert!(!red.alike(&print(0, 0, 220).unwrap()));
        assert!(!red.alike(&print(200, 0, 0).unwrap()));
        assert!(Print::from_bgra(&solid(0, 0, 200, 10)).is_none(), "nearly clear");
        assert!(Print::from_bgra(&[0; 12]).is_none(), "wrong size");
        // Colour under a clear pixel doesn't count.
        assert_eq!(Print::from_bgra(&solid(9, 9, 9, 255)).map(|p| p.0[3]), Some(255));
        let mut half = solid(0, 0, 200, 255);
        half[..LEN / 2].copy_from_slice(&solid(77, 77, 77, 0)[..LEN / 2]);
        let mut other = half.clone();
        other[..LEN / 2].copy_from_slice(&solid(5, 5, 5, 0)[..LEN / 2]);
        assert_eq!(Print::from_bgra(&half), Print::from_bgra(&other));
    }

    #[test]
    fn same_art_under_two_urls_counts_as_the_first() {
        let mut p = Prints::default();
        assert!(!p.add("https://a/team.png", print(0, 0, 200)));
        assert!(!p.add("https://b/blue.png", print(200, 0, 0)));
        assert!(p.add("https://c/made.webp", print(1, 0, 200)));
        assert!(p.add("https://d/again.webp", print(0, 1, 201)));
        assert!(!p.add("https://e/broken", None));
        assert!(!p.add("https://c/made.webp", print(200, 0, 0)), "kept as it was");
        let map = p.map();
        assert_eq!(map.len(), 2);
        assert_eq!(map["https://c/made.webp"], "https://a/team.png");
        assert_eq!(map["https://d/again.webp"], "https://a/team.png");
        // The same pictures in the same order give the same map.
        let mut q = Prints::default();
        for (u, c) in [("https://a/team.png", 200), ("https://c/made.webp", 200), ("https://d/again.webp", 201)] {
            q.add(u, print(0, 0, c));
        }
        assert_eq!(q.map(), map);
    }

    #[test]
    fn the_file_round_trips_and_is_capped() {
        let mut p = Prints::default();
        p.add("https://a/1.png", print(0, 0, 200));
        p.add("https://b/2.png", print(0, 0, 201));
        p.add("https://c/3.png", None);
        let back = Prints::from_json(&p.to_json());
        assert_eq!(back, p);
        assert_eq!(back.map()["https://b/2.png"], "https://a/1.png");
        assert!(Prints::from_json("not json").is_empty());
        assert_eq!(Prints::from_json(r#"[["https://x","zz","https://x"],["","","y"],["https://y","","https://y"]]"#).len(), 1);
        let mut big = Prints::default();
        for i in 0..MAX + 5 {
            big.add(&format!("https://x/{i}"), None);
        }
        assert_eq!(big.len(), MAX);
        assert!(!big.has("https://x/0") && big.has(&format!("https://x/{}", MAX + 4)));
        assert_eq!(Prints::from_json(&big.to_json()).len(), MAX);
    }
}
