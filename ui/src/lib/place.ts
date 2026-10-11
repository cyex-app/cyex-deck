// V101-04 Where popovers and tooltips go: next to what they belong to, flipped to the
// other side when there's no room, then kept fully inside the window. Every floating
// thing uses it: tooltips, menus, emote cards, previews and dropdowns.
// V102-03 It never lands on what it belongs to: with no room above or below it goes
// beside, and with no room anywhere it shrinks to the larger side and scrolls. It also
// shrinks to the window's width or height when it's bigger, and follows its own size.

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
  /** Set when it has to be smaller than it wants to fit. */
  maxWidth?: number;
  maxHeight?: number;
}

const OPPOSITE: Record<Side, Side> = { below: 'above', above: 'below', right: 'left', left: 'right' };
const ACROSS: Record<Side, Side[]> = { below: ['right', 'left'], above: ['right', 'left'], right: ['below', 'above'], left: ['below', 'above'] };

/** Less room than this isn't worth shrinking into: it goes over the anchor instead. */
const MIN_ROOM = 120;

/** A box with no size, e.g. where the pointer is. */
export function point(x: number, y: number): Rect {
  return { left: x, top: y, right: x, bottom: y };
}

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(v, hi));

export function place(anchor: Rect, size: Size, view: Size, o: PlaceOptions = {}): Placed {
  const { side = 'below', align = 'start', gap = 6, edge = 8 } = o;
  const along = (x: Side) => x === 'below' || x === 'above';
  const need = (x: Side) => (along(x) ? size.height : size.width);
  const room = (x: Side) =>
    ({
      below: view.height - edge - anchor.bottom - gap,
      above: anchor.top - gap - edge,
      right: view.width - edge - anchor.right - gap,
      left: anchor.left - gap - edge,
    })[x];
  // The preferred side, the other one, then beside: the first with room for all of it.
  // Else the larger of the first two, shrunk to fit if that leaves enough to use.
  const roomier = room(OPPOSITE[side]) > room(side) ? OPPOSITE[side] : side;
  const s = [side, OPPOSITE[side], ...ACROSS[side]].find((x) => room(x) >= need(x)) ?? roomier;
  const vertical = along(s);
  const fits = room(s) >= need(s);
  const mainLen = fits || room(s) < Math.min(need(s), MIN_ROOM) ? need(s) : room(s);
  const main = {
    below: anchor.bottom + gap,
    above: anchor.top - gap - mainLen,
    right: anchor.right + gap,
    left: anchor.left - gap - mainLen,
  }[s];

  const [a0, a1, want, span] = vertical
    ? [anchor.left, anchor.right, size.width, view.width]
    : [anchor.top, anchor.bottom, size.height, view.height];
  const len = Math.min(want, Math.max(0, span - 2 * edge));
  const far = span - edge - len;
  let cross = align === 'start' ? a0 : align === 'end' ? a1 - len : (a0 + a1 - len) / 2;
  // Running past an edge lines it up with the anchor's other end, the way menus open
  // leftwards near the right edge.
  if (align === 'start' && cross > far) cross = a1 - len;
  if (align === 'end' && cross < edge) cross = a0;

  const mainEnd = (vertical ? view.height : view.width) - edge - mainLen;
  const m = clamp(main, edge, Math.max(edge, mainEnd));
  const c = clamp(cross, edge, Math.max(edge, far));
  const out: Placed = vertical ? { left: c, top: m, side: s } : { left: m, top: c, side: s };
  const [mainMax, crossMax] = vertical ? (['maxHeight', 'maxWidth'] as const) : (['maxWidth', 'maxHeight'] as const);
  if (mainLen < need(s)) out[mainMax] = mainLen;
  if (len < want) out[crossMax] = len;
  return out;
}

export interface FloatOptions extends PlaceOptions {
  /** What it belongs to: an element, or a fixed box such as the pointer. */
  anchor: Element | Rect | null | undefined;
  /** As wide as the anchor, e.g. suggestions over the message box. */
  matchWidth?: boolean;
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
 * and again whenever its size or the window changes. When it has to be smaller than it
 * wants, it gets a max size and scrolls (V102-03). */
export function floating(node: HTMLElement, opts: FloatOptions) {
  let o = opts;
  const run = () => {
    const a = o.anchor;
    if (!a) return;
    const anchor = a instanceof Element ? a.getBoundingClientRect() : a;
    if (o.matchWidth) node.style.width = `${anchor.right - anchor.left}px`;
    // The size it wants: measured without the limits set last time.
    node.style.maxWidth = '';
    node.style.maxHeight = '';
    const size = { width: node.offsetWidth, height: node.offsetHeight };
    const p = place(anchor, size, { width: innerWidth, height: innerHeight }, o);
    const at = origin(node);
    node.style.right = 'auto';
    node.style.bottom = 'auto';
    node.style.left = `${p.left - at.x}px`;
    node.style.top = `${p.top - at.y}px`;
    node.style.maxWidth = p.maxWidth === undefined ? '' : `${p.maxWidth}px`;
    node.style.maxHeight = p.maxHeight === undefined ? '' : `${p.maxHeight}px`;
    node.style.overflow = p.maxWidth === undefined && p.maxHeight === undefined ? '' : 'auto';
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
