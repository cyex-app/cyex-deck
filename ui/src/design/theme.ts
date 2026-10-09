// Applies theme and accent (3.3, 3.4). Accent math lives in Rust (cyex_core::color);
// this file only writes the results into CSS variables.

import { themeIpc } from '../ipc/theme';
import type { AccentTokens } from '../ipc/bindings/AccentTokens';
import type { Appearance } from '../ipc/bindings/Appearance';
import type { Theme } from '../ipc/bindings/Theme';

const CACHE_KEY = 'cyex.theme';

interface Applied {
  theme: Theme;
  tokens: AccentTokens;
}

/** Paints last session's theme before settings load, so there's no flash. */
export function applyCachedTheme(): void {
  try {
    const cached = localStorage.getItem(CACHE_KEY);
    if (cached) write(JSON.parse(cached) as Applied);
  } catch {
    // No cache yet: the dark defaults in tokens.css stay.
  }
}

let latest = 0;

export async function applyAppearance(appearance: Appearance): Promise<void> {
  const request = ++latest;
  const tokens = await themeIpc.accentTokens(appearance.accent, appearance.theme);
  if (request !== latest) return; // a newer color arrived while this one was computing
  const applied: Applied = { theme: appearance.theme, tokens };
  write(applied);
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(applied));
  } catch {
    // Storage unavailable: next start just paints the defaults first.
  }
}

/** Accent tokens for a color the user is still picking, without saving it. */
export function previewAccent(accent: string, theme: Theme): Promise<AccentTokens> {
  return themeIpc.accentTokens(accent, theme);
}

function write({ theme, tokens }: Applied): void {
  const root = document.documentElement;
  root.dataset.theme = theme;
  for (const [key, value] of Object.entries(tokens)) {
    root.style.setProperty(`--${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`, value);
  }
}
