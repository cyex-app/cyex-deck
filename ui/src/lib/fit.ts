// V102-24 When a panel gets too small to use and where it goes instead. The sizes live
// here and nowhere else (logged in DECISIONS.md). A moved layout is never saved: the
// user's own (Settings 2.1 or dragged, V102-23) comes back once there's room again.
// V102-26 The mod view inside a tab is placed here too. V103-06 The mod view's and the stream
// manager's own panels fit through `lib/panels.ts` (`fitTree`), with their sizes there.

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
  /** V102-26 The mod view in a tab: one column of widgets, its header and one widget. */
  modviewW: 320,
  modviewH: 260,
};

/** V102-26 The channel header over a stream (StreamHeader and its border). The mod view
 * splits what's under it. */
export const STREAM_HEAD = 37;

/** V102-26 The mod view's share of the stream area until its divider is dragged. */
export const MODVIEW_SHARE = 0.5;

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

/** V102-26 A mod view beside a chat that has no stream. */
export const MODVIEW: Partner = { min: { w: MIN.modviewW, h: MIN.modviewH }, stacked: (box) => Math.max(MIN.chatH / box.h, 0.45) };

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

/** V102-26 Where a tile's mod view goes. */
export interface ModFit {
  /** Side by side, the mod view on the left; else the stream on top, the mod view below. */
  row: boolean;
  /** The mod view's share of what it splits with the stream. */
  share: number;
  /** The tab's chat steps aside for room (the mod view has its own, V102-22). */
  hideChat: boolean;
  /** No room even then: it pops out (V102-16 docks it back). */
  out: boolean;
}

export interface ModArea {
  tile: Box;
  /** The tile's chat as fitted (V102-24), or null when it doesn't show. */
  chat: { side: Side; ratio: number } | null;
  /** What it shares the area with: the stream (under its header), a clip, or nothing. */
  partner: 'stream' | 'clip' | null;
  /** The mod view's share, as dragged. */
  share: number;
}

/** What's left of the tile beside its chat, under the stream's header. */
function modSpace(a: ModArea, withChat: boolean): Box {
  let { w, h } = a.tile;
  if (withChat && a.chat) {
    if (across(a.chat.side)) w *= 1 - a.chat.ratio;
    else h *= 1 - a.chat.ratio;
  }
  return { w, h: a.partner === 'stream' ? h - STREAM_HEAD : h };
}

/** The mod view's share along `len`, or null when it and the stream can't both have their
 * least room, plus `margin`. */
function cut(len: number, want: number, own: number, other: number, margin: number): number | null {
  if (len < own + other + margin) return null;
  return Math.min(Math.max(want, own / len), 1 - other / len);
}

/** How the mod view splits `area`: side by side when it's wider than 16:9, else top and
 * bottom, or the other way when only that fits. `row` is the last answer: the shape has
 * to pass 16:9 by the hysteresis to turn. */
function split(area: Box, a: ModArea, margin: number, row?: boolean): { row: boolean; share: number } | null {
  if (!a.partner) return area.w >= MIN.modviewW + margin && area.h >= MIN.modviewH + margin ? { row: true, share: 1 } : null;
  const bias = row === undefined ? 0 : row ? -HYSTERESIS : HYSTERESIS;
  const wide = area.w - (area.h * 16) / 9 > bias;
  for (const r of wide ? [true, false] : [false, true]) {
    const [len, cross, own, other, need] = r
      ? [area.w, area.h, MIN.modviewW, MIN.streamW, Math.max(MIN.modviewH, MIN.streamH)]
      : [area.h, area.w, MIN.modviewH, MIN.streamH, Math.max(MIN.modviewW, MIN.streamW)];
    const s = cross >= need + margin ? cut(len, a.share, own, other, margin) : null;
    if (s !== null) return { row: r, share: s };
  }
  return null;
}

/** V102-26 The mod view inside a tile, in the stream's area: beside the tab's chat when
 * there's room, else in the chat's place too, else out in its own window. Showing the chat
 * again takes the hysteresis more room. `last` is the last answer. */
export function fitModview(a: ModArea, last: ModFit | null = null): ModFit {
  if (a.chat) {
    const s = split(modSpace(a, true), a, last?.hideChat ? HYSTERESIS : 0, last?.row);
    if (s) return { ...s, hideChat: false, out: false };
  }
  const s = split(modSpace(a, false), a, 0, last?.row);
  if (s) return { ...s, hideChat: a.chat !== null, out: false };
  return { row: last?.row ?? false, share: a.share, hideChat: false, out: true };
}
