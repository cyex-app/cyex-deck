// S1 Scrolls the settings column only. `scrollIntoView` also moves every scrolling ancestor,
// and the window with them (the title bar slid out of view), so the offset is worked out
// here and only the nearest scrolling parent moves.

/** The nearest ancestor that scrolls vertically. */
export function scrollParent(el: HTMLElement): HTMLElement | null {
  for (let p = el.parentElement; p; p = p.parentElement) {
    const { overflowY } = getComputedStyle(p);
    if ((overflowY === 'auto' || overflowY === 'scroll') && p.scrollHeight > p.clientHeight) return p;
  }
  return null;
}

/** Brings `el` into view inside its scrolling parent: centred, or just enough (`nearest`). */
export function scrollWithin(el: HTMLElement, block: 'center' | 'nearest', behavior: ScrollBehavior = 'auto'): void {
  const root = scrollParent(el);
  if (!root) return;
  const box = root.getBoundingClientRect();
  const r = el.getBoundingClientRect();
  const top = r.top - box.top + root.scrollTop;
  let to: number;
  if (block === 'center') {
    to = top - (root.clientHeight - r.height) / 2;
  } else if (r.top < box.top) {
    to = top;
  } else if (r.bottom > box.bottom) {
    to = top + r.height - root.clientHeight;
  } else {
    return;
  }
  root.scrollTo({ top: Math.max(0, to), behavior });
}
