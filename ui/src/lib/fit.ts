// V102-24 When a panel gets too small to use and where it goes instead. The sizes live
// here and nowhere else (logged in DECISIONS.md). A moved layout is never saved: the
// user's own (Settings 2.1 or dragged, V102-23) comes back once there's room again.

/** Smallest usable sizes, in CSS pixels. */
export const MIN = {
  /** Room to type in the message box, next to its emote and GIF buttons. */
  inputW: 140,
  /** One line in the message box. */
  inputH: 34,
  /** The message box with its buttons and padding. */
  chatW: 240,
  /** Header, message box and about five lines. */
  chatH: 200,
  /** The picture with its controls. */
  streamW: 240,
  streamH: 135,
  /** The stream manager's cards (V102-22): one column of them. */
  cardsW: 300,
  cardsH: 240,
};

/** How far past a limit the space has to grow before a moved layout goes back. */
export const HYSTERESIS = 48;

export type Side = 'left' | 'right' | 'top' | 'bottom';

export interface Box {
  w: number;
  h: number;
}

export interface TileFit {
  side: Side;
  /** The chat's share of the tile. */
  ratio: number;
  /** Turned the other way round to fit, not the user's layout. */
  moved: boolean;
}

/** 3.11 The chat's share in a tile's moved layout, beside the stream. */
const SIDE_SHARE = 0.28;

const across = (s: Side) => s === 'left' || s === 'right';

/** The chat's share along `len` once it has at least `chat` px, or null when the other
 * panel would get less than `other` px. `margin` is added to the chat's. */
function share(len: number, ratio: number, chat: number, other: number, margin: number): number | null {
  const want = Math.max(len * ratio, chat + margin);
  return len - want >= other ? Math.max(ratio, chat / len) : null;
}

/** Stacked: the stream at its full width's 16:9 height where it can, the chat below. */
function stackedShare(box: Box): number {
  return Math.max(MIN.chatH, box.h - (box.w * 9) / 16) / box.h;
}

/** What shares the space with the chat: its smallest size, and the chat's share when the
 * chat goes under it. */
export interface Partner {
  min: Box;
  stacked: (box: Box) => number;
}

export const STREAM: Partner = { min: { w: MIN.streamW, h: MIN.streamH }, stacked: stackedShare };

/** V102-22 The stream manager's cards. */
export const CARDS: Partner = { min: { w: MIN.cardsW, h: MIN.cardsH }, stacked: (box) => Math.max(MIN.chatH / box.h, 0.45) };

/** Where a tile's chat goes in `box` (stream and chat, or a clip beside a chat). The
 * user's side when both panels fit, the chat widened up to its minimum if needed; else
 * the other way round (chat below the stream, or beside one that's too short) when that
 * fits; else the user's side as it is. `moved` is the last answer, for the hysteresis. */
export function fitTile(box: Box, side: Side, ratio: number, moved = false, partner: Partner = STREAM): TileFit {
  const keep = { side, ratio, moved: false };
  if (box.w <= 0 || box.h <= 0) return keep;
  const row = across(side);
  const margin = moved ? HYSTERESIS : 0;
  const p = partner.min;
  const own = row ? share(box.w, ratio, MIN.chatW, p.w, margin) : share(box.h, ratio, MIN.chatH, p.h, margin);
  if (own !== null) return { side, ratio: own, moved: false };
  // Staying moved is easier than moving: the limit is `margin` lower.
  const alt = row
    ? share(box.h, partner.stacked(box), MIN.chatH, p.h, -margin)
    : share(box.w, SIDE_SHARE, MIN.chatW, p.w, -margin);
  if (alt !== null) return { side: row ? 'bottom' : 'right', ratio: alt, moved: true };
  // Neither fits: the user's way, with the chat as big as the other panel allows.
  const len = row ? box.w : box.h;
  const [chat, other] = row ? [MIN.chatW, p.w] : [MIN.chatH, p.h];
  const best = Math.min(Math.max(ratio, chat / len), Math.max(ratio, 1 - other / len));
  return { side, ratio: best, moved: false };
}

/** The least room a tile needs to be usable, whichever way round its panels go. */
export function tileNeed(stream: boolean, chat: boolean): Box {
  return {
    w: Math.max(chat ? MIN.chatW : 0, stream ? MIN.streamW : 0),
    h: Math.max(chat ? MIN.chatH : 0, stream ? MIN.streamH : 0),
  };
}

export interface ColumnNeed {
  /** Width share, as in the tab. */
  size: number;
  tiles: Box[];
}

/** U3 Whether a tab's columns stack into one (several chats one under the other): when a
 * column is too narrow for its tiles and every tile fits stacked. `stacked` is the last
 * answer, for the hysteresis. */
export function stackColumns(box: Box, columns: ColumnNeed[], stacked = false): boolean {
  if (columns.length < 2 || box.w <= 0 || box.h <= 0) return false;
  const margin = stacked ? HYSTERESIS : 0;
  const total = columns.reduce((s, c) => s + c.size, 0) || columns.length;
  const wide = columns.every((c) => {
    const need = Math.max(0, ...c.tiles.map((t) => t.w));
    return (box.w * (c.size || 1)) / total >= need + margin;
  });
  if (wide) return false;
  const all = columns.flatMap((c) => c.tiles);
  const high = box.h / all.length;
  return all.every((t) => high >= t.h - margin && box.w >= t.w);
}
