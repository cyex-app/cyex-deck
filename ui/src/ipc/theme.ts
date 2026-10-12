import { invoke } from '@tauri-apps/api/core';

import type { AccentTokens } from './bindings/AccentTokens';
import type { CustomFont } from './bindings/CustomFont';
import type { CustomTheme } from './bindings/CustomTheme';
import type { ThemeLook } from './bindings/ThemeLook';
import type { Theme } from './bindings/Theme';

export const themeIpc = {
  accentTokens: (accent: string, theme: Theme) => invoke<AccentTokens>('theme_accent_tokens', { accent, theme }),
  /** What the window paints: palette, accent and wallpaper of the theme in use. */
  look: () => invoke<ThemeLook>('theme_look'),
  /** U8 */
  presets: () => invoke<CustomTheme[]>('theme_presets'),
  /** U8 Adds a theme with a fresh id and switches to it; returns the id. */
  add: (theme: CustomTheme) => invoke<string>('theme_add', { theme }),
  code: (id: string) => invoke<string>('theme_code', { id }),
  import: (code: string) => invoke<string>('theme_import', { code }),
  /** File name in the app's wallpapers folder, or null when cancelled. */
  pickWallpaper: () => invoke<string | null>('theme_pick_wallpaper'),
};

/** V101-10 Chat font files the user adds. */
export const fontsIpc = {
  /** Picks a font file and makes it the chat font; null when cancelled. */
  add: () => invoke<CustomFont | null>('fonts_add'),
  /** Forgets an added font and deletes its copy. */
  remove: (name: string) => invoke<void>('fonts_remove', { name }),
};
