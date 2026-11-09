// V110-U69 When a hover opens or closes a submenu. Pure: the timers and the menu are handed in,
// so the timing and the "heading toward the flyout" test run without a window.

/** Wait before a hovered submenu row opens its flyout. */
export const OPEN_MS = 150;
/** Wait before leaving a row closes the flyout; a pointer heading toward it gets this too. */
export const CLOSE_MS = 300;

export interface Pt {
  x: number;
  y: number;
}

export interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

const cross = (a: Pt, b: Pt, p: Pt) => (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);

/** Whether `p` is inside the triangle a, b, c (edges count). */
export function inTriangle(p: Pt, a: Pt, b: Pt, c: Pt): boolean {
  const d1 = cross(a, b, p);
  const d2 = cross(b, c, p);
  const d3 = cross(c, a, p);
  const neg = d1 < 0 || d2 < 0 || d3 < 0;
  const pos = d1 > 0 || d2 > 0 || d3 > 0;
  return !(neg && pos);
}

/**
 * The safe triangle: from where the pointer left the row (`from`) to the two near corners of the
 * flyout. A pointer inside it is on its way there, so the rows it crosses don't close the flyout.
 * `side` is the side of the parent the flyout opened on.
 */
export function headingToward(from: Pt, now: Pt, child: Box, side: 'right' | 'left'): boolean {
  const x = side === 'right' ? child.left : child.right;
  return inTriangle(now, from, { x, y: child.top }, { x, y: child.bottom });
}

export interface Timers {
  set(fn: () => void, ms: number): number;
  clear(id: number): void;
}

export interface Row {
  /** Depth of the panel the row is in (0 = the first menu). */
  depth: number;
  index: number;
  /** The row opens a submenu. */
  sub: boolean;
  /** The row whose flyout is open in this panel, if any. */
  open: number | null;
  /** The pointer is heading toward the open flyout (headingToward). */
  toward?: boolean;
  /** Something in a flyout is being used (a typed field, an armed item): leave it open. */
  busy?: boolean;
}

export interface IntentHandlers {
  /** Open the flyout of row `index` in panel `depth`, closing anything deeper. */
  show(depth: number, index: number): void;
  /** Close every panel deeper than `depth`. */
  hide(depth: number): void;
}

export interface Intent {
  /** The pointer came onto a row. */
  enter(row: Row): void;
  /** The pointer is inside a flyout: whatever was pending is off. */
  panel(): void;
  /** Drop what's pending (the menu closed). */
  cancel(): void;
  readonly pending: boolean;
}

export function createIntent(h: IntentHandlers, timers: Timers, openMs = OPEN_MS, closeMs = CLOSE_MS): Intent {
  let id: number | null = null;
  const cancel = () => {
    if (id !== null) timers.clear(id);
    id = null;
  };
  const later = (ms: number, fn: () => void) => {
    cancel();
    id = timers.set(() => {
      id = null;
      fn();
    }, ms);
  };
  return {
    enter(r) {
      // Nothing to change: this row's flyout is the one open, or no flyout is open for a plain row.
      if (r.busy || (r.sub ? r.open === r.index : r.open === null)) return cancel();
      if (r.sub) later(r.toward ? closeMs : openMs, () => h.show(r.depth, r.index));
      else later(closeMs, () => h.hide(r.depth));
    },
    panel: cancel,
    cancel,
    get pending() {
      return id !== null;
    },
  };
}
