// Applies the theme (3.3, 3.4, U8). Colors are worked out in Rust (cyex_core::color and
// cyex_core::palette); this file only writes them into CSS variables and sets up the
// wallpaper layer (tokens.css, ChatView).

import { themeIpc } from '../ipc/theme';
import type { AccentTokens } from '../ipc/bindings/AccentTokens';
import type { Appearance } from '../ipc/bindings/Appearance';
import type { ThemeLook } from '../ipc/bindings/ThemeLook';
import type { Theme } from '../ipc/bindings/Theme';
import { reduced } from '../lib/motion';
import { fillCache } from './fillCache';

const CACHE_KEY = 'cyex.look';

/** Palette variables the last custom theme set, so a plain theme can drop them again. */
let customKeys: string[] = [];

/** Paints last session's theme before settings load, so there's no flash. */
export function applyCachedTheme(): void {
  try {
    const cached = localStorage.getItem(CACHE_KEY);
    if (cached) {
      const look = JSON.parse(cached) as ThemeLook;
      painted = themeKey(look);
      write(look);
    }
  } catch {
    // No cache yet: the dark defaults in tokens.css stay.
  }
}

let latest = 0;

/** Repaints after an appearance change; Rust reads the saved settings itself. */
export async function applyAppearance(_appearance: Appearance): Promise<void> {
  const request = ++latest;
  const look = await themeIpc.look();
  if (request !== latest) return; // a newer change arrived while this one was computing
  paint(look);
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(look));
  } catch {
    // Storage unavailable: next start just paints the defaults first.
  }
}

/** Accent tokens for a color the user is still picking, without saving it. */
export function previewAccent(accent: string, theme: Theme): Promise<AccentTokens> {
  return themeIpc.accentTokens(accent, theme);
}

/** V106 ra-3 The accent fill the UI would paint for an accent on a base (theme previews). Worked out
 *  against the base's backgrounds, so a custom theme's own page is only approximated. */
export const accentFill = fillCache((accent: string, theme: Theme) => previewAccent(accent, theme).then((t) => t.accent));

/** What a theme switch changes (not the accent alone, which is dragged live). */
const themeKey = (look: ThemeLook) => JSON.stringify([look.theme, look.palette?.tokens ?? null, look.wallpaper]);
let painted = '';

/** V106 s1-3 A switch to another theme cross-fades for 150 ms; none under Reduce motion.
 *  V106 ra-4 Color transitions are off while it runs, so nothing drifts under the fade. */
function paint(look: ThemeLook): void {
  const key = themeKey(look);
  const switched = painted !== '' && key !== painted;
  painted = key;
  const doc = document as Document & {
    startViewTransition?: (update: () => void) => { finished: Promise<unknown> };
  };
  if (!switched || reduced() || !doc.startViewTransition) return write(look);
  ensureFadeStyle();
  const root = document.documentElement;
  root.classList.add('theme-fading');
  const done = () => root.classList.remove('theme-fading');
  doc.startViewTransition(() => write(look)).finished.then(done, done);
}

let fadeStyle = false;
function ensureFadeStyle(): void {
  if (fadeStyle) return;
  fadeStyle = true;
  const style = document.createElement('style');
  style.textContent =
    '::view-transition-old(root),::view-transition-new(root){animation-duration:150ms}' +
    '.theme-fading,.theme-fading *,.theme-fading *::before,.theme-fading *::after{transition:none!important}';
  document.head.append(style);
}

const cssName = (key: string) => `--${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`;

function write(look: ThemeLook): void {
  const root = document.documentElement;
  // U7 Overlays stay dark whatever the theme, and never show a wallpaper.
  const overlay = root.classList.contains('overlay');
  root.dataset.theme = overlay ? 'dark' : look.theme;
  for (const key of customKeys) root.style.removeProperty(`--${key}`);
  customKeys = [];
  for (const [key, value] of Object.entries((!overlay && look.palette?.tokens) || {})) {
    root.style.setProperty(`--${key}`, value);
    customKeys.push(key);
  }
  for (const [key, value] of Object.entries(look.accent)) root.style.setProperty(cssName(key), value);

  const w = overlay ? null : look.wallpaper;
  if (!w) {
    delete root.dataset.wall;
    return;
  }
  root.dataset.wall = w.area;
  root.style.setProperty('--wall-image', `url(${JSON.stringify(w.url)})`);
  root.style.setProperty('--wall-opacity', String(1 - w.dim / 100));
  root.style.setProperty('--wall-blur', `${w.blur}px`);
  root.style.setProperty('--wall-size', w.fit === 'tile' ? 'auto' : w.fit);
  root.style.setProperty('--wall-repeat', w.fit === 'tile' ? 'repeat' : 'no-repeat');
}
