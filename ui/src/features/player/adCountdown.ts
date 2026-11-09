// V110-U57 The ad break note: the seconds left, counted in the window from what Rust
// told (ms left when it said so). No duration known: the plain note.

export const AD_NOTE = 'Ad break. The stream comes back right after.';

/** `m:ss` for the time left, rounded up so 0:00 only shows at the very end. */
export function adClock(ms: number): string {
  const s = Math.max(0, Math.ceil(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** The moment the break ends on this window's clock, or null when nothing is known. */
export function endsAt(leftMs: number | null | undefined, now: number): number | null {
  return typeof leftMs === 'number' && Number.isFinite(leftMs) && leftMs > 0 ? now + leftMs : null;
}

/** Ms left at `now`, or null without an end. */
export function msLeft(end: number | null, now: number): number | null {
  return end === null ? null : Math.max(0, end - now);
}
