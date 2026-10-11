// V103-11 Smooth scrolling the way Chatterino does it (c2 `Scrollbar::setDesiredValue`,
// `ChannelView::wheelEvent`): a wheel notch moves the target 180 px, the view follows in
// 150 ms on an OutCubic curve, and notches during a glide add to its target while the
// curve starts again from where the view is. Pure math; `smoothScroll.ts` drives the page.

/** Chatterino's scroll animation length (`currentValueAnimation_.setDuration(150)`). */
export const SCROLL_MS = 150;
/** px per wheel notch: Chatterino's `angleDelta().y() * 1.5`, 120 per notch. */
export const NOTCH_PX = 180;
/** Chatterino's `QEasingCurve::OutCubic`, for CSS and Web Animations. */
export const EASE_OUT_CUBIC = 'cubic-bezier(0.33, 1, 0.68, 1)';
/** Positions this close are the same place (Chatterino's `areClose`), in px. */
const CLOSE = 1;

export const easeOutCubic = (t: number): number => 1 - (1 - t) ** 3;

/** A glide of one scroll area. Times are `performance.now()`. */
export interface Glide {
  from: number;
  to: number;
  start: number;
  /** Chatterino's `atBottom_`: set by the target, so the glide keeps to the bottom while
   * content grows under it. */
  bottom: boolean;
}

/** `top` within 0 and `max`; nothing to scroll (`max` below 0) is 0. */
export function clampTop(top: number, max: number): number {
  return Math.min(Math.max(top, 0), Math.max(max, 0));
}

export function atBottom(top: number, max: number): boolean {
  return Math.max(max, 0) - top <= CLOSE;
}

/** Whether a target at `top` can still move `by` px. */
export function moves(top: number, by: number, max: number): boolean {
  return Math.abs(clampTop(top + by, max) - top) > CLOSE;
}

/**
 * Wheel notches in an event, down positive. `wheelDeltaY` is 120 per notch, up positive
 * (Qt's `angleDelta`). 0 when it isn't a notched wheel: touchpads and free-spinning wheels
 * move in small steps already and a page-at-a-time wheel keeps its pages, so those scroll
 * natively.
 */
export function notches(wheelDeltaY: number | undefined, deltaY: number, deltaMode: number): number {
  if (deltaMode === 2) return 0;
  let n = 0;
  if (wheelDeltaY) n = wheelDeltaY % 120 === 0 ? -wheelDeltaY / 120 : 0;
  else if (deltaMode === 1) n = deltaY / 3;
  return Math.sign(n) === Math.sign(deltaY) ? n : 0;
}

/**
 * The target `by` px further: from the playing glide's target, so notches add up, or from
 * `top` when nothing plays. The curve starts again from `top`, where the view is. Null when
 * the target can't move that way (the wheel goes on to the area around it).
 */
export function aim(g: Glide | null, top: number, by: number, max: number, now: number): Glide | null {
  const base = g ? g.to : top;
  if (!moves(base, by, max)) return null;
  const to = clampTop(base + by, max);
  return { from: top, to, start: now, bottom: atBottom(to, max) };
}

/**
 * The glide after something else moved the view `moved` px since the last frame: the
 * browser keeping lines in place as old ones drop off or grow, keys, code that jumps. It
 * carries on by the same amount from there. One headed for the bottom stays headed for it.
 */
export function carry(g: Glide, moved: number, max: number): Glide {
  const shift = Math.abs(moved) > CLOSE ? moved : 0;
  const to = g.bottom ? Math.max(max, 0) : clampTop(g.to + shift, max);
  return { from: g.from + shift, to, start: g.start, bottom: g.bottom || atBottom(to, max) };
}

/** Where the view is at `now`. */
export function positionAt(g: Glide, now: number): number {
  const t = Math.min(Math.max((now - g.start) / SCROLL_MS, 0), 1);
  return g.from + (g.to - g.from) * easeOutCubic(t);
}

/** One frame: where to put the view, and whether the glide is over. */
export function frame(g: Glide, now: number, max: number): { top: number; done: boolean } {
  return { top: clampTop(positionAt(g, now), max), done: now - g.start >= SCROLL_MS };
}
