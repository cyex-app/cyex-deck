// V103-12 How long new chat lines take to slide in. Kept light for fast chats: a slide never
// lasts longer than the gap since the last lines came, one too short to see is skipped,
// and more than half the chat at once just shows. So the newest line is never more than
// one short slide behind, and stick-to-bottom never waits on it. Length and curve are
// Chatterino's (`scrollMath.ts`), so lines slide like the wheel glides.

import { SCROLL_MS } from '../../lib/scrollMath.ts';

/** A slide when chat is calm, in ms. */
export const GLIDE_MS = SCROLL_MS;
/** Shorter than this isn't worth playing: the lines just show. */
export const MIN_GLIDE_MS = 60;

/**
 * `distance`: px the lines move up. `viewport`: the chat's height in px. `sinceLast`: ms
 * since the previous lines came. Returns the slide's length in ms; 0 jumps.
 */
export function glideMs(distance: number, viewport: number, sinceLast: number): number {
  if (!(distance >= 1) || distance > viewport / 2) return 0;
  const ms = Math.min(GLIDE_MS, sinceLast);
  return ms >= MIN_GLIDE_MS ? Math.round(ms) : 0;
}
