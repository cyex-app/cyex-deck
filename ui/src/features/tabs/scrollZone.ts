// V110-U51 The tab bar's own scrollbar: a press on it must not move the window (the bar is a
// drag region in the title bar), and the wheel on it or just beside it scrolls the bar
// instead of stepping through tabs.

/** Extra px left of the scrollbar that still counts as the scrollbar for the wheel. */
export const WHEEL_REACH = 8;

/** Box of a scrollable element, as `inScrollbar` needs it. */
export interface ScrollBox {
  right: number;
  /** Scrollbar width: offsetWidth - clientWidth (no borders on the bar). */
  bar: number;
  /** Whether there is anything to scroll. */
  scrolls: boolean;
}

/** Whether `x` is on the vertical scrollbar, or within `reach` px to its left. */
export function inScrollbar(box: ScrollBox, x: number, reach = 0): boolean {
  if (!box.scrolls || box.bar <= 0) return false;
  return x >= box.right - box.bar - reach && x <= box.right;
}

export function scrollBox(el: HTMLElement): ScrollBox {
  return {
    right: el.getBoundingClientRect().right,
    bar: el.offsetWidth - el.clientWidth,
    scrolls: el.scrollHeight > el.clientHeight,
  };
}
