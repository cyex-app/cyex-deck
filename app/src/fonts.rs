//! V101-10 Font files the user adds for chat. They're copied into `fonts\` next to the
//! settings and served to the page as `cyexfont://`.

use std::fs;
use std::path::{Path, PathBuf};

use cyex_settings::Partition;
use cyex_settings::schema::{CustomFont, FONT_TYPES, General, font_name};
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager, UriSchemeContext, Wry};

use crate::settings::Settings;

pub const SCHEME: &str = "cyexfont";
/// Fonts with every script can be big; more than this is never a chat font.
const MAX_FONT_BYTES: u64 = 30 * 1024 * 1024;
/// Names the bundled fonts use (`design/fonts.css`, U13 the app font), so an added file
/// doesn't hide one.
const BUNDLED: [&str; 6] = [
    "IBM Plex Sans",
    "Inter",
    "Roboto",
    "Open Sans",
    "Lato",
    "JetBrains Mono",
];

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
    app.path()
        .config_dir()
        .ok()
        .map(|d| d.join("cyex-viewer").join("fonts"))
}

/// Copies a font file into the fonts folder. Named after the file, made unique.
pub fn add(app: &AppHandle, path: &Path) -> Result<CustomFont, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| FONT_TYPES.contains(&e.as_str()))
        .ok_or("Pick a TTF, OTF, WOFF or WOFF2 font file.")?;
    let size = fs::metadata(path).map_err(|_| "Couldn't read that file.")?.len();
    if size > MAX_FONT_BYTES {
        return Err("That font file is over 30 MB.".into());
    }
    let g: General = app.state::<Settings>().read(Partition::General);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let taken: Vec<&str> = g.fonts.iter().map(|f| f.name.as_str()).chain(BUNDLED).collect();
    let name = unique_name(&family_of(&stem), &taken);
    let dir = fonts_dir(app).ok_or("Couldn't find the app data folder.")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = format!("{}.{ext}", cyex_settings::schema::random_id());
    fs::copy(path, dir.join(&file)).map_err(|_| "Couldn't copy that font.")?;
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
}
