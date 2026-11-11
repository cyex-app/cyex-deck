// V110-B10 Theatre is fullscreen and covers the taskbar. At the bottom edge the window shrinks
// to the work area so the taskbar shows; it covers the screen again once the pointer has been
// well above the bottom for a moment. Pure timing rules; the class feeds them pointer events.

/** The bottom edge, in px from the window's bottom. */
export const EDGE_PX = 2;
/** The pointer must be further than this from the bottom to count as back. */
export const BACK_PX = 80;
/** ...for this long, in ms. */
export const BACK_AFTER_MS = 400;

export interface Room {
  /** The window leaves room for the taskbar. */
  on: boolean;
  /** When the pointer first stayed clear of the bottom; `null` otherwise. */
  since: number | null;
}

export const OFF: Room = { on: false, since: null };

/** `fromBottom`: the pointer's distance from the window's bottom, `null` when it is outside
 *  the window (over the taskbar, say, which stays). */
export function roomStep(prev: Room, fromBottom: number | null, now: number): Room {
  if (fromBottom !== null && fromBottom <= EDGE_PX) return { on: true, since: null };
  if (!prev.on) return OFF;
  if (fromBottom === null || fromBottom <= BACK_PX) return { on: true, since: null };
  const since = prev.since ?? now;
  return now - since >= BACK_AFTER_MS ? OFF : { on: true, since };
}

/** A2-7 Which band the pointer is in; a move inside one band changes nothing. */
export function bandOf(fromBottom: number | null): 'out' | 'edge' | 'near' | 'far' {
  if (fromBottom === null) return 'out';
  return fromBottom <= EDGE_PX ? 'edge' : fromBottom <= BACK_PX ? 'near' : 'far';
}

/** A2-7 Whether a pointer move needs a step: only when the band changed (a running timer keeps its clock). */
export function pointerChanged(prev: number | null, next: number | null): boolean {
  return bandOf(prev) !== bandOf(next);
}

/** ms until the window covers the screen again, or `null` when nothing is pending. */
export function backIn(r: Room, now: number): number | null {
  return r.on && r.since !== null ? Math.max(0, r.since + BACK_AFTER_MS - now) : null;
}
