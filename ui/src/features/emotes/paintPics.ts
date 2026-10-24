// 1.37 A paint's pictures load before its rules go in. A CSS picture that fails once stays
// failed for the page's life (the rule keeps its broken image and never asks again), so a
// single CDN miss left that name plain in the window until a restart. Pure, so it's testable.

import type { PaintStyle } from '../../ipc/bindings/PaintStyle';

/** Waits before each new try of a failed picture. */
export const RETRY_MS = [2_000, 15_000, 60_000];

/** The pictures a paint's rules ask for, moving and still. */
export function paintUrls(p: PaintStyle): string[] {
  const urls = new Set<string>();
  for (const css of [p.css, p.still ?? '']) for (const m of css.matchAll(/url\("([^"]+)"\)/g)) urls.add(m[1]);
  return [...urls];
}

/** Loads every picture, trying a failed one again after each wait. True if all loaded. */
export async function loadAll(
  urls: string[],
  load: (url: string) => Promise<boolean>,
  wait: (ms: number) => Promise<void>,
  retries = RETRY_MS,
): Promise<boolean> {
  let left = urls;
  for (const ms of [0, ...retries]) {
    if (ms) await wait(ms);
    const ok = await Promise.all(left.map(load));
    left = left.filter((_, i) => !ok[i]);
    if (!left.length) return true;
  }
  return false;
}

/** A browser load of one picture; it lands in the page's cache for the CSS. */
export function loadPicture(url: string): Promise<boolean> {
  return new Promise((done) => {
    const img = new Image();
    img.onload = () => done(true);
    img.onerror = () => done(false);
    img.src = url;
  });
}
