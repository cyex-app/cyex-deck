// Scrolls the nearest scrolling parent only. `scrollIntoView` also moves every scrolling
// ancestor, and the window with them (the title bar slid out of view, the page root moved
// by a few pixels), so the offset is worked out here instead.

type Axis = 'y' | 'x';

/** The nearest ancestor that scrolls on `axis` (vertically by default). */
export function scrollParent(el: HTMLElement, axis: Axis = 'y'): HTMLElement | null {
  for (let p = el.parentElement; p; p = p.parentElement) {
    const cs = getComputedStyle(p);
    const [overflow, scroll, client] = axis === 'y' ? [cs.overflowY, p.scrollHeight, p.clientHeight] : [cs.overflowX, p.scrollWidth, p.clientWidth];
    if ((overflow === 'auto' || overflow === 'scroll') && scroll > client) return p;
  }
  return null;
}

/** Brings `el` into view inside its scrolling parent: centred, or just enough (`nearest`). */
export function scrollWithin(el: HTMLElement, block: 'center' | 'nearest', behavior: ScrollBehavior = 'auto', axis: Axis = 'y'): void {
  const root = scrollParent(el, axis);
  if (!root) return;
  const y = axis === 'y';
  const box = root.getBoundingClientRect();
  const r = el.getBoundingClientRect();
  const [bStart, bEnd, rStart, rEnd, size, cur] = y
    ? [box.top, box.bottom, r.top, r.bottom, r.height, root.scrollTop]
    : [box.left, box.right, r.left, r.right, r.width, root.scrollLeft];
  const view = y ? root.clientHeight : root.clientWidth;
  const start = rStart - bStart + cur;
  let to: number;
  if (block === 'center') {
    to = start - (view - size) / 2;
  } else if (rStart < bStart) {
    to = start;
  } else if (rEnd > bEnd) {
    to = start + size - view;
  } else {
    return;
  }
  root.scrollTo(y ? { top: Math.max(0, to), behavior } : { left: Math.max(0, to), behavior });
}
