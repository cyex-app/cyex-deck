// V110-B7 In theatre the window controls (the title bar) hide and slide in when the pointer
// touches the top of the screen or keyboard focus moves into them. Pure timing rules; the
// component feeds them pointer and focus events.

/** How close to the top edge the pointer must be, in px. */
export const EDGE_PX = 4;
/** How long the bar stays after the pointer leaves it, in ms. */
export const HIDE_AFTER_MS = 600;

export interface Reveal {
  shown: boolean;
  /** When the pointer and focus left it; `null` while they are on it or it is hidden. */
  since: number | null;
}

export interface RevealInput {
  /** The pointer's distance from the top of the window, `null` when it left the window. */
  y: number | null;
  /** The pointer is over the bar. */
  overBar: boolean;
  /** Keyboard focus is inside the bar. */
  focusIn: boolean;
  now: number;
}

/** A2-7 Whether the pointer alone asks for the bar; a move that doesn't change this needs no step. */
export function pointerNear(y: number | null, overBar: boolean): boolean {
  return (y !== null && y <= EDGE_PX) || overBar;
}

export const HIDDEN: Reveal = { shown: false, since: null };

/** The next state. Touching the top edge, the bar or focus shows it at once; leaving starts the 600 ms. */
export function revealStep(prev: Reveal, i: RevealInput): Reveal {
  const near = (i.y !== null && i.y <= EDGE_PX) || i.overBar || i.focusIn;
  if (near) return { shown: true, since: null };
  if (!prev.shown) return HIDDEN;
  const since = prev.since ?? i.now;
  return i.now - since >= HIDE_AFTER_MS ? HIDDEN : { shown: true, since };
}

/** ms until a shown bar hides, or `null` when nothing is pending. */
export function hideIn(r: Reveal, now: number): number | null {
  return r.shown && r.since !== null ? Math.max(0, r.since + HIDE_AFTER_MS - now) : null;
}
