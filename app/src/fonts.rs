//! V101-10 Font files the user adds for chat. They're copied into `fonts\` next to the
//! settings and served to the page as `cyexfont://`.

use std::fs;
use std::path::{Path, PathBuf};

use cyex_settings::Partition;
use cyex_settings::schema::{CustomFont, FONT_TYPES, General, font_name};
use serde::Serialize;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager, UriSchemeContext, Wry};
use ts_rs::TS;

use crate::settings::Settings;

pub const SCHEME: &str = "cyexfont";
/// Fonts with every script can be big; more than this is never a chat font.
const MAX_FONT_BYTES: u64 = 30 * 1024 * 1024;
/// The app font (U13); "App font" in the list.
const APP_FONT: &str = "IBM Plex Sans";
/// V101-10 Chat fonts that come with the app (`ui/src/design/fonts.css`).
pub const BUNDLED: [&str; 5] = ["Inter", "Roboto", "Open Sans", "Lato", "JetBrains Mono"];

/// V105-13 Where a font in the list comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FontKind {
    Bundled,
    Added,
    Installed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FontChoice {
    pub name: String,
    pub kind: FontKind,
}

/// V105-13 Every chat font there is, A-Z, each name once: an added file wins over a
/// bundled font, a bundled one over an installed one of the same name. The app font is
/// "App font", not in here.
pub fn choices(added: &[String], installed: &[String]) -> Vec<FontChoice> {
    let mut out: Vec<FontChoice> = Vec::new();
    let mut push = |name: &str, kind: FontKind| {
        if !out.iter().any(|c| c.name.eq_ignore_ascii_case(name)) {
            out.push(FontChoice { name: name.to_owned(), kind });
        }
    };
    for n in added {
        push(n, FontKind::Added);
    }
    for n in BUNDLED {
        push(n, FontKind::Bundled);
    }
    for n in installed {
        if !n.eq_ignore_ascii_case(APP_FONT) {
            push(n, FontKind::Installed);
        }
    }
    out.sort_by_key(|c| c.name.to_lowercase());
    out
}

/// Files no saved font points to any more.
pub fn init(app: &AppHandle) {
    let g: General = app.state::<Settings>().read(Partition::General);
    if let Some(dir) = fonts_dir(app) {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !g.fonts.iter().any(|f| f.file == name) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

fn fonts_dir(app: &AppHandle) -> Option<PathBuf> {
    crate::paths::config(app).ok().map(|d| d.join("fonts"))
}

/// Copies a font file into the fonts folder. Named after the file, made unique.
pub fn add(app: &AppHandle, path: &Path) -> Result<CustomFont, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| FONT_TYPES.contains(&e.as_str()))
        .ok_or("Pick a TTF, OTF, WOFF or WOFF2 font file.")?;
    let size = fs::metadata(path).map_err(|_| "Couldn’t read that file.")?.len();
    if size > MAX_FONT_BYTES {
        return Err("That font file is over 30 MB.".into());
    }
    let g: General = app.state::<Settings>().read(Partition::General);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let taken: Vec<&str> = g.fonts.iter().map(|f| f.name.as_str()).chain(BUNDLED)
        .chain([APP_FONT])
        .collect();
    let name = unique_name(&family_of(&stem), &taken);
    let dir = fonts_dir(app).ok_or("Couldn’t find the app data folder.")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = format!("{}.{ext}", cyex_settings::schema::random_id());
    fs::copy(path, dir.join(&file)).map_err(|_| "Couldn’t copy that font.")?;
    Ok(CustomFont { name, file })
}

/// Deletes an added font's file.
pub fn remove_file(app: &AppHandle, font: &CustomFont) {
    if font.is_file()
        && let Some(dir) = fonts_dir(app)
    {
        let _ = fs::remove_file(dir.join(&font.file));
    }
}

/// "OpenDyslexic-Regular" → "OpenDyslexic", "my_font bold" → "my font bold".
fn family_of(stem: &str) -> String {
    let spaced = stem.replace(['_', '-'], " ");
    let words: Vec<&str> = spaced.split_whitespace().collect();
    let words = match words.split_last() {
        Some((last, rest)) if !rest.is_empty() && last.eq_ignore_ascii_case("regular") => rest,
        _ => &words[..],
    };
    let name = font_name(&words.join(" "));
    if name.is_empty() { "My font".into() } else { name }
}

fn unique_name(base: &str, taken: &[&str]) -> String {
    let free = |n: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(n));
    if free(base) {
        return base.to_owned();
    }
    (2..)
        .map(|i| format!("{base} {i}"))
        .find(|n| free(n))
        .unwrap_or_default()
}

/// `cyexfont://localhost/<file>`: an added font file, nothing else.
pub fn handle(ctx: UriSchemeContext<'_, Wry>, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let name = request.uri().path().trim_start_matches('/').to_owned();
    let probe = CustomFont {
        file: name.clone(),
        ..Default::default()
    };
    let file = probe
        .is_file()
        .then(|| fonts_dir(ctx.app_handle()))
        .flatten()
        .and_then(|dir| fs::read(dir.join(&name)).ok());
    match file {
        Some(bytes) => {
            let mime = match name.rsplit('.').next() {
                Some("otf") => "font/otf",
                Some("woff") => "font/woff",
                Some("woff2") => "font/woff2",
                _ => "font/ttf",
            };
            Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
                // Pages load fonts from another origin only with CORS.
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(bytes)
                .unwrap_or_default()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_come_from_the_file() {
        assert_eq!(family_of("OpenDyslexic-Regular"), "OpenDyslexic");
        assert_eq!(family_of("my_font bold"), "my font bold");
        assert_eq!(family_of("Regular"), "Regular");
        assert_eq!(family_of("--"), "My font");
        assert_eq!(family_of("a\";}b"), "ab");
    }

    #[test]
    fn names_never_hide_another_font() {
        assert_eq!(unique_name("Comic", &["Inter"]), "Comic");
        assert_eq!(unique_name("inter", &["Inter"]), "inter 2");
        assert_eq!(unique_name("Inter", &["Inter", "Inter 2"]), "Inter 3");
    }

    #[test]
    fn the_list_has_each_font_once_az() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let list = choices(&s(&["Lato", "My Font"]), &s(&["arial", "Lato", "IBM Plex Sans", "Zed"]));
        let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            ["arial", "Inter", "JetBrains Mono", "Lato", "My Font", "Open Sans", "Roboto", "Zed"]
        );
        let kind = |n: &str| list.iter().find(|c| c.name == n).unwrap().kind;
        assert_eq!(kind("Lato"), FontKind::Added);
        assert_eq!(kind("arial"), FontKind::Installed);
        assert_eq!(kind("Inter"), FontKind::Bundled);
    }

    #[test]
    fn every_bundled_font_is_in_fonts_css() {
        const CSS: &str = include_str!("../../ui/src/design/fonts.css");
        for name in BUNDLED {
            assert!(CSS.contains(&format!("font-family: '{name}'")), "{name} missing in fonts.css");
        }
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/design/fonts");
        for part in CSS.split("url('./fonts/").skip(1) {
            let file = part.split('\'').next().unwrap();
            assert!(dir.join(file).is_file(), "{file} missing");
        }
    }
}
