//! U8 Theme share codes: one theme as text to paste somewhere else. A wallpaper from a
//! file stays on this PC; a wallpaper link travels with the code.
//! V109-T01 Links to cyex.app (`https://cyex.app/t/<id>`) are read here too.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::error::{Error, Result};
use crate::schema::{CustomTheme, valid_link_id};

const PREFIX: &str = "cyextheme1.";
/// Codes longer than this aren't themes (a theme with a long link is ~3 KB).
const MAX_CODE_CHARS: usize = 8_000;
/// V109-T01 Where a shared theme's link points.
pub const LINK_BASE: &str = "https://cyex.app/t/";
const LINK_HOST_PATH: &str = "cyex.app/t/";

pub fn to_code(theme: &CustomTheme) -> String {
    let json = serde_json::to_vec(&theme.shareable()).expect("theme serializes");
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
    let theme: CustomTheme = serde_json::from_slice(&bytes).map_err(|_| Error::BadCode)?;
    let mut theme = theme.shareable();
    theme.sanitize();
    Ok(theme)
}

/// V109-T01 `https://cyex.app/t/<id>`
pub fn link_of(id: &str) -> String {
    format!("{LINK_BASE}{id}")
}

/// V109-T01 The id in a pasted link (`https://cyex.app/t/k3f9x2ab`, `cyex.app/t/k3f9x2ab`,
/// with or without `www.`, a trailing slash or query) or a bare id. None for anything else,
/// old theme codes included.
pub fn parse_link(text: &str) -> Option<String> {
    let t = text.trim().to_ascii_lowercase();
    if valid_link_id(&t) {
        return Some(t);
    }
    let rest = t
        .strip_prefix("https://")
        .or_else(|| t.strip_prefix("http://"))
        .unwrap_or(&t);
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let id = rest.strip_prefix(LINK_HOST_PATH)?;
    let id = id.split(['/', '?', '#']).next().unwrap_or_default();
    valid_link_id(id).then(|| id.to_owned())
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

    #[test]
    fn v109_t01_codes_never_carry_a_link() {
        let mut t = builtin_themes()[1].clone();
        let sent = t.clone();
        t.record_share("k3f9x2ab", 3, &sent);
        t.link_source("m4n5p6q7", 2, "2026/11/05");
        let code = to_code(&t);
        let body = URL_SAFE_NO_PAD.decode(code.strip_prefix(PREFIX).unwrap()).unwrap();
        let json = String::from_utf8(body).unwrap();
        assert!(!json.contains("link") && !json.contains("k3f9x2ab") && !json.contains("m4n5p6q7"));
        assert!(from_code(&code).unwrap().link.is_empty());
    }

    #[test]
    fn v109_t01_links_and_ids() {
        for ok in [
            "https://cyex.app/t/k3f9x2ab",
            "  https://cyex.app/t/k3f9x2ab/ ",
            "http://cyex.app/t/k3f9x2ab",
            "https://www.cyex.app/t/k3f9x2ab?ref=x",
            "cyex.app/t/k3f9x2ab",
            "CYEX.APP/T/K3F9X2AB",
            "cyex.app/t/k3f9x2ab#top",
            "k3f9x2ab",
            " k3f9x2ab\n",
        ] {
            assert_eq!(parse_link(ok).as_deref(), Some("k3f9x2ab"), "{ok}");
        }
        for bad in [
            "",
            "k3f9x2a",
            "k3f9x2abc",
            "k3f9x2a0",
            "https://cyex.app/t/",
            "https://cyex.app/t/k3f9x2a1",
            "https://evil.app/t/k3f9x2ab",
            "https://cyex.app.evil/t/k3f9x2ab",
            "https://cyex.app/x/k3f9x2ab",
            "cyextheme1.e30",
        ] {
            assert_eq!(parse_link(bad), None, "{bad}");
        }
        assert_eq!(link_of("k3f9x2ab"), "https://cyex.app/t/k3f9x2ab");
    }
}
