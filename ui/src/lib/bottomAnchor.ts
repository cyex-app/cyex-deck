// A scroll list that keeps its bottom edge still when its own viewport changes height (the
// usercard's top part grows or shrinks above it). Pinned at the newest line: stay pinned.
// Scrolled up: keep the same distance from the bottom, so the view grows or shrinks from the top.

/** Closer than this to the end counts as "at the bottom". */
export const PIN_PX = 24;

export interface Metrics {
  scrollHeight: number;
  scrollTop: number;
  clientHeight: number;
}

/** How far the viewport's bottom edge is above the end of the content (0 at the end). */
export function gapBelow(m: Metrics): number {
  return Math.max(0, m.scrollHeight - m.scrollTop - m.clientHeight);
}

export function isPinned(m: Metrics): boolean {
  return gapBelow(m) < PIN_PX;
}

/** The scrollTop that keeps `gap` between the viewport's bottom edge and the content's end. */
export function anchoredTop(m: Pick<Metrics, 'scrollHeight' | 'clientHeight'>, gap: number, pinned: boolean): number {
  return Math.max(0, m.scrollHeight - m.clientHeight - (pinned ? 0 : gap));
}
