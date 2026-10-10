// V101-04 Where popovers and tooltips go: next to what they belong to, flipped to the
// other side when there's no room, then kept fully inside the window. Every floating
// thing uses it: tooltips, menus, emote cards, previews and dropdowns.

export type Side = 'below' | 'above' | 'right' | 'left';
export type Align = 'start' | 'center' | 'end';

export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface PlaceOptions {
  /** Preferred side of the anchor. */
  side?: Side;
  /** Along the other axis: lined up with the anchor's start, middle or end. */
  align?: Align;
  /** Space between the anchor and the popover. */
  gap?: number;
  /** Space kept free along the window's edges. */
  edge?: number;
}

export interface Placed {
  left: number;
  top: number;
  side: Side;
}

const OPPOSITE: Record<Side, Side> = { below: 'above', above: 'below', right: 'left', left: 'right' };

/** A box with no size, e.g. where the pointer is. */
export function point(x: number, y: number): Rect {
  return { left: x, top: y, right: x, bottom: y };
}

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(v, hi));

export function place(anchor: Rect, size: Size, view: Size, o: PlaceOptions = {}): Placed {
  const { side = 'below', align = 'start', gap = 6, edge = 8 } = o;
  const vertical = side === 'below' || side === 'above';
  const need = vertical ? size.height : size.width;
  const room = (s: Side) =>
    ({
      below: view.height - edge - anchor.bottom - gap,
      above: anchor.top - gap - edge,
      right: view.width - edge - anchor.right - gap,
      left: anchor.left - gap - edge,
    })[s];
  const s = room(side) < need && room(OPPOSITE[side]) > room(side) ? OPPOSITE[side] : side;
  const main = {
    below: anchor.bottom + gap,
    above: anchor.top - gap - size.height,
    right: anchor.right + gap,
    left: anchor.left - gap - size.width,
  }[s];

  const [a0, a1, len, span] = vertical
    ? [anchor.left, anchor.right, size.width, view.width]
    : [anchor.top, anchor.bottom, size.height, view.height];
  const far = span - edge - len;
  let cross = align === 'start' ? a0 : align === 'end' ? a1 - len : (a0 + a1 - len) / 2;
  // Running past an edge lines it up with the anchor's other end, the way menus open
  // leftwards near the right edge.
  if (align === 'start' && cross > far) cross = a1 - len;
  if (align === 'end' && cross < edge) cross = a0;

  const mainEnd = (vertical ? view.height : view.width) - edge - need;
  const m = clamp(main, edge, Math.max(edge, mainEnd));
  const c = clamp(cross, edge, Math.max(edge, far));
  return vertical ? { left: c, top: m, side: s } : { left: m, top: c, side: s };
}

export interface FloatOptions extends PlaceOptions {
  /** What it belongs to: an element, or a fixed box such as the pointer. */
  anchor: Element | Rect | null | undefined;
}

/** Top-left of the node's containing block on screen. */
function origin(node: HTMLElement): { x: number; y: number } {
  if (getComputedStyle(node).position === 'fixed') return { x: 0, y: 0 };
  const parent = node.offsetParent as HTMLElement | null;
  if (!parent) return { x: 0, y: 0 };
  const r = parent.getBoundingClientRect();
  return {
    x: r.left + parent.clientLeft - parent.scrollLeft,
    y: r.top + parent.clientTop - parent.scrollTop,
  };
}

/** `use:floating={{ anchor, side }}` on a fixed or absolute node: placed with `place`,
 * and again whenever its size or the window changes. */
export function floating(node: HTMLElement, opts: FloatOptions) {
  let o = opts;
  const run = () => {
    const a = o.anchor;
    if (!a) return;
    const anchor = a instanceof Element ? a.getBoundingClientRect() : a;
    const size = { width: node.offsetWidth, height: node.offsetHeight };
    const p = place(anchor, size, { width: innerWidth, height: innerHeight }, o);
    const at = origin(node);
    node.style.right = 'auto';
    node.style.bottom = 'auto';
    node.style.left = `${p.left - at.x}px`;
    node.style.top = `${p.top - at.y}px`;
    node.dataset.side = p.side;
  };
  const sized = new ResizeObserver(run);
  sized.observe(node);
  addEventListener('resize', run);
  run();
  return {
    update(next: FloatOptions) {
      o = next;
      run();
    },
    destroy() {
      sized.disconnect();
      removeEventListener('resize', run);
    },
  };
}
