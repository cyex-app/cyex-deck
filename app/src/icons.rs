//! App mark for windows and the tray (3.12). The dark variant (light bars) goes on dark
//! taskbars, the light variant on light ones.

use tauri::Theme;
use tauri::image::Image;

const DARK: &[u8] = include_bytes!("../icons/window-dark.png");
const LIGHT: &[u8] = include_bytes!("../icons/window-light.png");

pub fn for_theme(theme: Theme) -> Image<'static> {
    let bytes = match theme {
        Theme::Light => LIGHT,
        _ => DARK,
    };
    Image::from_bytes(bytes).expect("bundled icon decodes")
}

pub fn apply(window: &tauri::Window) {
    let theme = window.theme().unwrap_or(Theme::Dark);
    let _ = window.set_icon(for_theme(theme));
}
