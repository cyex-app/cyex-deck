// U8 Names and checks for the theme editor.
import type { Appearance } from '../../ipc/bindings/Appearance';
import type { CustomTheme } from '../../ipc/bindings/CustomTheme';
import type { Theme } from '../../ipc/bindings/Theme';

/** Theme colors (`cyex_core::palette::KEYS`) with the names the editor shows. */
export const COLOR_NAMES: readonly (readonly [string, string])[] = [
  ['bg', 'Background'],
  ['surface', 'Panels'],
  ['elevated', 'Menus and cards'],
  ['border', 'Lines'],
  ['text', 'Text'],
  ['muted', 'Secondary text'],
  ['dim', 'Faint text'],
  ['red', 'Red'],
  ['yellow', 'Yellow'],
  ['green', 'Green'],
  ['cyan', 'Cyan'],
];

const NAMES = new Map(COLOR_NAMES);

/** "Text and Faint text were nudged …" for the colors Rust had to change. */
export function adjustedNote(keys: readonly string[]): string {
  const names = keys.map((k) => NAMES.get(k) ?? k);
  if (!names.length) return '';
  const list = names.length === 1 ? names[0] : `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`;
  return `${list} ${names.length === 1 ? 'was' : 'were'} nudged so everything stays readable.`;
}

/** An https link Rust will take as a wallpaper (no quotes, brackets or spaces). */
export function isLink(source: string): boolean {
  return /^https:\/\/[^\s"'()\\]+$/.test(source) && source.length <= 2048;
}

/** T2 `#rrggbb` from what was typed (`#abc`, `abc` and `#AABBCC` all work); null when it isn't a color. */
export function parseHex(text: string): string | null {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(text.trim());
  if (!m) return null;
  const h = m[1].toLowerCase();
  return `#${h.length === 3 ? [...h].map((c) => c + c).join('') : h}`;
}

/** T1 The palette's own color for `key` when it is a plain hex, else null. */
export function appliedHex(value: string | undefined): string | null {
  return value && /^#[0-9a-f]{6}$/i.test(value) ? value.toLowerCase() : null;
}

/** T1 "Kept dark so text stays readable": why a picked color isn't the one painted. */
export function keptNote(key: string, base: 'dark' | 'light'): string {
  return ['bg', 'surface', 'elevated', 'border'].includes(key)
    ? `Kept ${base} so text stays readable`
    : 'Nudged so it stays readable';
}

export type ThemeChoice =
  | { kind: 'plain'; base: Theme }
  | { kind: 'builtin'; theme: CustomTheme }
  | { kind: 'custom'; theme: CustomTheme };

/** Colors for the little previews of Dark and Light; keep in sync with tokens.css. */
export const PLAIN_PREVIEW: Record<Theme, { label: string; bg: string; surface: string; text: string }> = {
  dark: { label: 'Dark', bg: '#1e1f23', surface: '#26272c', text: '#e8e8ea' },
  light: { label: 'Light', bg: '#f3f3f4', surface: '#ffffff', text: '#1b1c20' },
};

export const DEFAULT_ACCENT = '#145aff';

/** V106-02 What `appearance` shows now; mirrors `Appearance::active_theme` in Rust (custom ids win). */
export function choiceOf(a: Appearance, builtins: readonly CustomTheme[]): ThemeChoice {
  if (a.themeId) {
    const custom = a.themes.find((t) => t.id === a.themeId);
    if (custom) return { kind: 'custom', theme: custom };
    const builtin = builtins.find((t) => t.id === a.themeId);
    if (builtin) return { kind: 'builtin', theme: builtin };
  }
  return { kind: 'plain', base: a.theme };
}

/** The accent in use. */
export function accentOf(a: Appearance, c: ThemeChoice): string {
  if (c.kind === 'custom') return c.theme.accent;
  if (c.kind === 'builtin') return a.accents[c.theme.id] ?? c.theme.accent;
  return a.accent;
}

/** The accent "Default" goes back to. */
export function defaultAccentOf(c: ThemeChoice): string {
  return c.kind === 'builtin' ? c.theme.accent : DEFAULT_ACCENT;
}

/** V106-03 "New": a copy of what shows now (Rust gives it an id and switches to it). */
export function newThemeFrom(a: Appearance, c: ThemeChoice): CustomTheme {
  const accent = accentOf(a, c);
  if (c.kind === 'plain') {
    return { id: '', name: 'My theme', base: c.base, colors: {}, accent, wallpaper: blankWallpaper() };
  }
  const t = c.theme;
  return {
    id: '',
    name: `${t.name} copy`,
    base: t.base,
    colors: { ...t.colors },
    accent,
    wallpaper: c.kind === 'custom' ? { ...t.wallpaper } : blankWallpaper(),
  };
}

function blankWallpaper(): CustomTheme['wallpaper'] {
  return { source: '', area: 'window', dim: 70, blur: 0, fit: 'cover' };
}
