//! U8 Theme share codes: one theme as text to paste somewhere else. A wallpaper from a
//! file stays on this PC; a wallpaper link travels with the code.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::error::{Error, Result};
use crate::schema::CustomTheme;

const PREFIX: &str = "cyextheme1.";
/// Codes longer than this aren't themes (a theme with a long link is ~3 KB).
const MAX_CODE_CHARS: usize = 8_000;

pub fn to_code(theme: &CustomTheme) -> String {
    let mut t = theme.clone();
    t.id.clear();
    if t.wallpaper.is_file() {
        t.wallpaper.source.clear();
    }
    let json = serde_json::to_vec(&t).expect("theme serializes");
    format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json))
}

/// The theme in a code, cleaned up, without an id (the caller gives it one).
pub fn from_code(code: &str) -> Result<CustomTheme> {
    let body: String = code.split_whitespace().collect();
    if body.len() > MAX_CODE_CHARS {
        return Err(Error::BadCode);
    }
    let body = body.strip_prefix(PREFIX).ok_or(Error::BadCode)?;
    let bytes = URL_SAFE_NO_PAD.decode(body).map_err(|_| Error::BadCode)?;
    let mut theme: CustomTheme = serde_json::from_slice(&bytes).map_err(|_| Error::BadCode)?;
    theme.id.clear();
    if theme.wallpaper.is_file() {
        theme.wallpaper.source.clear();
    }
    theme.sanitize();
    Ok(theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::builtin_themes;

    #[test]
    fn round_trip() {
        let mut t = builtin_themes()[0].clone();
        t.id = "abc".into();
        t.wallpaper.source = "https://example.com/wall.jpg".into();
        let back = from_code(&to_code(&t)).unwrap();
        assert!(back.id.is_empty());
        assert_eq!(back.name, t.name);
        assert_eq!(back.colors, t.colors);
        assert_eq!(back.wallpaper.source, t.wallpaper.source);
    }

    #[test]
    fn local_wallpapers_stay_home() {
        let mut t = builtin_themes()[2].clone();
        t.wallpaper.source = "abc123.png".into();
        let code = to_code(&t);
        assert!(from_code(&code).unwrap().wallpaper.source.is_empty());
    }

    #[test]
    fn spaces_are_ignored_and_junk_refused() {
        let code = to_code(&builtin_themes()[1].clone());
        let (a, b) = code.split_at(20);
        assert!(from_code(&format!(" {a}\n{b} ")).is_ok());
        assert!(matches!(from_code("hello"), Err(Error::BadCode)));
        assert!(matches!(from_code("cyextheme1.!!!"), Err(Error::BadCode)));
        assert!(matches!(from_code("cyex1.e30"), Err(Error::BadCode)));
    }
}
