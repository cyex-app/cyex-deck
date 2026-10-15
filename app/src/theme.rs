//! The theme in use (3.3, U8): plain dark or light, or a custom theme with its palette,
//! accent and wallpaper. Wallpaper files live in `wallpapers\` next to the settings and
//! are served to the page as `cyexpic://`.

use std::fs;
use std::path::{Path, PathBuf};

use cyex_core::color::{self, AccentTokens, Theme};
use cyex_core::palette::{self, Palette};
use cyex_settings::Partition;
use cyex_settings::schema::{Appearance, CustomTheme, WallpaperArea, WallpaperFit};
use serde::Serialize;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::window::Color;
use tauri::{AppHandle, Manager, UriSchemeContext, Wry};
use ts_rs::TS;

use crate::settings::Settings;

pub const SCHEME: &str = "cyexpic";
/// Bigger pictures aren't worth the memory a full-window background costs.
const MAX_WALLPAPER_BYTES: u64 = 20 * 1024 * 1024;

/// Everything a window needs to paint the theme.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThemeLook {
    pub theme: Theme,
    /// Set for custom themes; plain dark and light use `tokens.css` as is.
    pub palette: Option<Palette>,
    pub accent: AccentTokens,
    pub wallpaper: Option<WallpaperLook>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WallpaperLook {
    pub url: String,
    pub area: WallpaperArea,
    pub dim: u8,
    pub blur: u8,
    pub fit: WallpaperFit,
}

pub fn look(app: &AppHandle) -> ThemeLook {
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    look_of(&a)
}

fn look_of(a: &Appearance) -> ThemeLook {
    let Some(t) = a.active_theme() else {
        return ThemeLook {
            theme: a.theme,
            palette: None,
            accent: color::accent_tokens_on(&a.accent, a.theme, a.theme.builtin_backgrounds()),
            wallpaper: None,
        };
    };
    let p = palette::palette(t.base, &t.colors);
    let w = &t.wallpaper;
    let url = if w.is_link() {
        Some(w.source.clone())
    } else if w.is_file() {
        Some(picture_url(&w.source))
    } else {
        None
    };
    ThemeLook {
        theme: t.base,
        accent: palette::accent(&p, t.base, &t.accent),
        palette: Some(p),
        wallpaper: url.map(|url| WallpaperLook {
            url,
            area: w.area,
            dim: w.dim,
            blur: w.blur,
            fit: w.fit,
        }),
    }
}

/// The page background as a window color, so new windows don't flash before the UI paints.
pub fn background(app: &AppHandle) -> Color {
    let look = look(app);
    let bg = look
        .palette
        .as_ref()
        .and_then(|p| color::Rgb::parse(&p.bg))
        .unwrap_or(look.theme.builtin_backgrounds()[0]);
    Color(bg.r, bg.g, bg.b, 0xff)
}

/// Name colors and the accent are made readable against the custom theme's backgrounds.
pub fn on_settings(app: &AppHandle) {
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    for base in [Theme::Dark, Theme::Light] {
        let custom = a
            .active_theme()
            .filter(|t| t.base == base)
            .map(|t| palette::palette(t.base, &t.colors).backgrounds());
        color::set_backgrounds(base, custom);
    }
}

pub fn init(app: &AppHandle) {
    on_settings(app);
    // Pictures no theme uses any more (deleted themes, replaced wallpapers).
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    if let Some(dir) = wallpapers_dir(app) {
        let used: Vec<&str> = a.themes.iter().map(|t| t.wallpaper.source.as_str()).collect();
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !used.contains(&name.as_str()) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

fn wallpapers_dir(app: &AppHandle) -> Option<PathBuf> {
    crate::paths::config(app).ok().map(|d| d.join("wallpapers"))
}

/// The page's address for a wallpaper file. WebView2 serves custom schemes as
/// `http://<scheme>.localhost/`.
fn picture_url(file: &str) -> String {
    format!("http://{SCHEME}.localhost/{file}")
}

/// Copies a picture the user picked into the wallpapers folder; returns its new name.
pub fn add_wallpaper(app: &AppHandle, path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif"))
        .ok_or("Pick a PNG, JPG, WebP or GIF picture.")?;
    let size = fs::metadata(path).map_err(|_| "Couldn’t read that file.")?.len();
    if size > MAX_WALLPAPER_BYTES {
        return Err("That picture is over 20 MB. Pick a smaller one.".into());
    }
    let dir = wallpapers_dir(app).ok_or("Couldn’t find the app data folder.")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("{}.{ext}", cyex_settings::schema::random_id());
    fs::copy(path, dir.join(&name)).map_err(|_| "Couldn’t copy that picture.")?;
    Ok(name)
}

/// `cyexpic://localhost/<file>`: a wallpaper file, nothing else.
pub fn handle(ctx: UriSchemeContext<'_, Wry>, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let name = request.uri().path().trim_start_matches('/').to_owned();
    let probe = cyex_settings::schema::Wallpaper {
        source: name.clone(),
        ..Default::default()
    };
    let file = probe
        .is_file()
        .then(|| wallpapers_dir(ctx.app_handle()))
        .flatten()
        .and_then(|dir| fs::read(dir.join(&name)).ok());
    match file {
        Some(bytes) => {
            let mime = match name.rsplit('.').next() {
                Some("png") => "image/png",
                Some("webp") => "image/webp",
                Some("gif") => "image/gif",
                _ => "image/jpeg",
            };
            Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
                .body(bytes)
                .unwrap_or_default()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new())
            .unwrap_or_default(),
    }
}

/// A theme ready to add: cleaned, with a fresh id.
pub fn fresh(mut theme: CustomTheme) -> CustomTheme {
    theme.id = cyex_settings::schema::random_id();
    theme.sanitize();
    theme
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_settings::Schema;
    use cyex_settings::schema::preset_themes;

    #[test]
    fn plain_themes_have_no_palette() {
        let look = look_of(&Appearance::default());
        assert!(look.palette.is_none() && look.wallpaper.is_none());
        assert_eq!(look.accent.accent, "#145aff");
    }

    #[test]
    fn custom_theme_brings_palette_and_wallpaper() {
        let mut t = fresh(preset_themes().remove(2));
        t.wallpaper.source = "abc.webp".into();
        let mut a = Appearance {
            theme_id: t.id.clone(),
            themes: vec![t],
            ..Appearance::default()
        };
        a.sanitize();
        let look = look_of(&a);
        assert_eq!(look.theme, Theme::Light);
        assert_eq!(look.palette.unwrap().bg, "#f4efe6");
        assert_eq!(look.wallpaper.unwrap().url, "http://cyexpic.localhost/abc.webp");
    }
}
