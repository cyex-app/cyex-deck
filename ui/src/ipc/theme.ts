import { invoke } from '@tauri-apps/api/core';

import type { AccentTokens } from './bindings/AccentTokens';
import type { CustomTheme } from './bindings/CustomTheme';
import type { LinkUpdate } from './bindings/LinkUpdate';
import type { ThemeLook } from './bindings/ThemeLook';
import type { ThemePreview } from './bindings/ThemePreview';

/** V110-A7 `note` says why a theme from a code came without its wallpaper. */
export interface AddedTheme {
  id: string;
  note: string | null;
}
import type { Theme } from './bindings/Theme';

export const themeIpc = {
  accentTokens: (accent: string, theme: Theme) => invoke<AccentTokens>('theme_accent_tokens', { accent, theme }),
  /** What the window paints: palette, accent and wallpaper of the theme in use. */
  look: () => invoke<ThemeLook>('theme_look'),
  /** V106-02 Midnight, Forest, Dusk, Ocean, Ember. */
  builtins: () => invoke<CustomTheme[]>('theme_builtins'),
  /** U8 Adds a theme with a fresh id and switches to it; returns the id. */
  add: (theme: CustomTheme) => invoke<string>('theme_add', { theme }),
  /** U8 The old offline share code. */
  code: (id: string) => invoke<string>('theme_code', { id }),
  /** File name in the app's wallpapers folder, or null when cancelled. */
  pickWallpaper: () => invoke<string | null>('theme_pick_wallpaper'),

  // V109-T01 Links on cyex.app.
  /** Uploads (first time) and returns the link. */
  share: (id: string) => invoke<string>('theme_share', { id }),
  /** Pushes the theme to the same link; returns the new version. */
  shareUpdate: (id: string) => invoke<number>('theme_share_update', { id }),
  shareStop: (id: string) => invoke<void>('theme_share_stop', { id }),
  /** True when this PC has the theme's edit key. */
  canEdit: (id: string) => invoke<boolean>('theme_share_can_edit', { id }),
  /** A pasted link or old code, to show before adding. */
  preview: (text: string) => invoke<ThemePreview>('theme_preview', { text }),
  addPreview: (preview: ThemePreview) => invoke<AddedTheme>('theme_add_preview', { preview }),
  report: (sourceId: string, reason: string) => invoke<void>('theme_report', { sourceId, reason }),
  linkUpdate: (id: string, replace: boolean) => invoke<LinkUpdate>('theme_link_update', { id, replace }),
  linkRestore: (id: string, index: number) => invoke<void>('theme_link_restore', { id, index }),
  linkIgnore: (id: string) => invoke<void>('theme_link_ignore', { id }),
};
