// V108-S2 Geometry and keys for LayoutPicker: one small window drawing, 360x220.

export type Side = 'left' | 'right' | 'top' | 'bottom';
export type Rect = { x: number; y: number; w: number; h: number };
export type Pt = [number, number];

export const W = 360;
export const H = 220;
export const TITLE = 18;
/** The window under the title bar. */
export const BODY: Rect = { x: 0, y: TITLE, w: W, h: H - TITLE };

const BAR_ACROSS = 44; // a side bar's width
const BAR_ALONG = 24; // a top or bottom bar's height
const CHAT_SHARE = 0.36;

export const SIDES: readonly Side[] = ['top', 'bottom', 'left', 'right'];

/** The tab bar on `side`, and what is left of the body for player and chat. */
export function barSplit(side: Side): { bar: Rect; rest: Rect } {
  const { x, y, w, h } = BODY;
  switch (side) {
    case 'left':
      return { bar: { x, y, w: BAR_ACROSS, h }, rest: { x: x + BAR_ACROSS, y, w: w - BAR_ACROSS, h } };
    case 'right':
      return { bar: { x: x + w - BAR_ACROSS, y, w: BAR_ACROSS, h }, rest: { x, y, w: w - BAR_ACROSS, h } };
    case 'top':
      return { bar: { x, y, w, h: BAR_ALONG }, rest: { x, y: y + BAR_ALONG, w, h: h - BAR_ALONG } };
    case 'bottom':
      return { bar: { x, y: y + h - BAR_ALONG, w, h: BAR_ALONG }, rest: { x, y, w, h: h - BAR_ALONG } };
  }
}

/** Chat on `side` of `area`, and the player in the rest. */
export function chatSplit(side: Side, area: Rect): { chat: Rect; player: Rect } {
  const { x, y, w, h } = area;
  const cw = Math.round(w * CHAT_SHARE);
  const ch = Math.round(h * CHAT_SHARE);
  switch (side) {
    case 'left':
      return { chat: { x, y, w: cw, h }, player: { x: x + cw, y, w: w - cw, h } };
    case 'right':
      return { chat: { x: x + w - cw, y, w: cw, h }, player: { x, y, w: w - cw, h } };
    case 'top':
      return { chat: { x, y, w, h: ch }, player: { x, y: y + ch, w, h: h - ch } };
    case 'bottom':
      return { chat: { x, y: y + h - ch, w, h: ch }, player: { x, y, w, h: h - ch } };
  }
}

// Hit zones stay put while the drawing changes: a mitered frame for the bar, a cross for chat.
const FRAME_X = 72;
const FRAME_Y = 44;
const INNER: Rect = { x: FRAME_X, y: TITLE + FRAME_Y, w: W - 2 * FRAME_X, h: H - TITLE - 2 * FRAME_Y };

/** Corners of the zone that picks `side` for the tab bar: the frame around the window. */
export function tabZone(side: Side): Pt[] {
  const { x, y, w, h } = INNER;
  const { x: ox, y: oy, w: ow, h: oh } = BODY;
  const tl: Pt = [ox, oy];
  const tr: Pt = [ox + ow, oy];
  const br: Pt = [ox + ow, oy + oh];
  const bl: Pt = [ox, oy + oh];
  const itl: Pt = [x, y];
  const itr: Pt = [x + w, y];
  const ibr: Pt = [x + w, y + h];
  const ibl: Pt = [x, y + h];
  switch (side) {
    case 'top':
      return [tl, tr, itr, itl];
    case 'right':
      return [tr, br, ibr, itr];
    case 'bottom':
      return [br, bl, ibl, ibr];
    case 'left':
      return [bl, tl, itl, ibl];
  }
}

/** Corners of the zone that picks `side` for chat: the middle cut by its diagonals. */
export function chatZone(side: Side): Pt[] {
  const { x, y, w, h } = INNER;
  const c: Pt = [x + w / 2, y + h / 2];
  const tl: Pt = [x, y];
  const tr: Pt = [x + w, y];
  const br: Pt = [x + w, y + h];
  const bl: Pt = [x, y + h];
  switch (side) {
    case 'top':
      return [tl, tr, c];
    case 'right':
      return [tr, br, c];
    case 'bottom':
      return [br, bl, c];
    case 'left':
      return [bl, tl, c];
  }
}

/** An arrow key names the side it points at: the radio it picks. Null for any other key. */
export function sideForKey(key: string): Side | null {
  switch (key) {
    case 'ArrowLeft':
      return 'left';
    case 'ArrowRight':
      return 'right';
    case 'ArrowUp':
      return 'top';
    case 'ArrowDown':
      return 'bottom';
    default:
      return null;
  }
}

/** The caption for a pick: "Tabs on the left". */
export function sayPick(kind: 'tabs' | 'chat', side: Side): string {
  const where = side === 'top' || side === 'bottom' ? `at the ${side}` : `on the ${side}`;
  return `${kind === 'tabs' ? 'Tabs' : 'Chat'} ${where}`;
}

export const toPoints = (pts: Pt[]) => pts.map(([a, b]) => `${a},${b}`).join(' ');
export const toClip = (pts: Pt[]) =>
  `polygon(${pts.map(([a, b]) => `${((a / W) * 100).toFixed(3)}% ${((b / H) * 100).toFixed(3)}%`).join(', ')})`;
