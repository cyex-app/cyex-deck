// V107-P4 Where a confirm callout sits and how long it waits: under its button, the notch
// pointing at the button's middle, inside the window.
// V108-C21 It is a bubble: never over the control it confirms (so a second click on the
// same spot can't confirm), the notch on the press point, flips above at the window's
// bottom, and the 3 s timer pauses while the pointer is on it.

import type { Rect } from '../lib/place';

/** Height of the notch between the button and the box. */
export const NOTCH = 5;
/** The callout goes away by itself after this long. */
export const CONFIRM_MS = 3000;

/** The box's left and top, and the notch's x inside it. Centered under the button when it
 * fits, else pushed inside the window; the notch stays on the button. */
export function calloutAt(
  anchor: Rect,
  width: number,
  viewWidth: number,
  edge = 6,
  /** V108-C21 The press point's x, when known: the notch points there (kept on the control). */
  pressX?: number,
): { left: number; top: number; notch: number } {
  const mid = pressX === undefined ? (anchor.left + anchor.right) / 2 : Math.max(anchor.left, Math.min(pressX, anchor.right));
  const left = Math.max(edge, Math.min(mid - width / 2, viewWidth - edge - width));
  const notch = Math.max(NOTCH + 3, Math.min(mid - left, width - NOTCH - 3));
  return { left: Math.round(left), top: Math.round(anchor.bottom + NOTCH + 1), notch: Math.round(notch) };
}

/** V107-P5 Over a menu item instead: the box ends 1 px into the item's top edge, so the
 * pointer can go from the item to the box without crossing the item above, and the notch
 * points down into the item. Under it when there's no room above. */
export function calloutAbove(
  anchor: Rect,
  width: number,
  height: number,
  viewWidth: number,
  edge = 6,
): { left: number; top: number; notch: number; below: boolean } {
  const x = calloutAt(anchor, width, viewWidth, edge);
  const top = anchor.top + 1 - height;
  if (top < edge) return { ...x, top: Math.round(anchor.bottom - 1), below: true };
  return { ...x, top: Math.round(top), below: false };
}

/** V108-C21 Under the control, or above it when the window's bottom is too close. The box
 * never overlaps the control, so the pointer has to move to reach it. */
export function bubbleAt(
  anchor: Rect,
  width: number,
  height: number,
  viewWidth: number,
  viewHeight: number,
  edge = 6,
  pressX?: number,
): { left: number; top: number; notch: number; below: boolean } {
  const x = calloutAt(anchor, width, viewWidth, edge, pressX);
  const fitsBelow = x.top + height <= viewHeight - edge;
  const upTop = anchor.top - NOTCH - 1 - height;
  if (!fitsBelow && upTop >= edge) return { ...x, top: Math.round(upTop), below: false };
  return { ...x, below: true };
}

export interface Timers {
  set: (run: () => void, ms: number) => unknown;
  clear: (id: unknown) => void;
  /** Milliseconds clock for pause; Date.now when missing. */
  now?: () => number;
}

const REAL: Timers = {
  set: (run, ms) => setTimeout(run, ms),
  clear: (id) => clearTimeout(id as ReturnType<typeof setTimeout>),
};

export interface Expiry {
  cancel: () => void;
  /** V108-C21 Holds the clock (the pointer is on the bubble); the time left is kept. */
  pause: () => void;
  /** Runs the time that was left. */
  resume: () => void;
}

/** Calls `end` once after `ms`, unless canceled first. Can be paused and resumed. */
export function expiry(end: () => void, ms = CONFIRM_MS, t: Timers = REAL): Expiry {
  const now = t.now ?? Date.now;
  let live = true;
  let id: unknown = null;
  let started = 0;
  let left = ms;
  const fire = () => {
    if (live) {
      live = false;
      end();
    }
  };
  const arm = () => {
    started = now();
    id = t.set(fire, left);
  };
  arm();
  return {
    cancel() {
      if (!live) return;
      live = false;
      if (id !== null) t.clear(id);
      id = null;
    },
    pause() {
      if (!live || id === null) return;
      t.clear(id);
      id = null;
      left = Math.max(0, left - (now() - started));
    },
    resume() {
      if (!live || id !== null) return;
      arm();
    },
  };
}
