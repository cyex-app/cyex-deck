// F13 (v1.04 batch 2b) A tab's columns of tiles (U3) as one panel tree, so `layout` in
// `lib/panels.ts` places every tile absolutely, the way panels are placed since V103-05.
// The tiles stay in one list in a fixed order in the page; moving one only changes its box,
// so no stream in the tab restarts.

import { type Box, clampSizes, type Divider, isSplit, leaf, minOf, type Node, shares, split } from '../../lib/panels.ts';

export interface GridSizes {
  columns: number[];
  rows: number[][];
}

/** The tab's columns, left to right, each its tiles top to bottom (`Tab.columns`). */
export type Columns = readonly { tiles: readonly { id: string }[] }[];

/** The tree to lay out: a row of columns, each a column of tiles; stacked (V102-24) every
 * tile one under the other, in equal shares. */
export function gridTree(columns: Columns, sizes: GridSizes, stacked: boolean): Node | null {
  const ids = columns.flatMap((c) => c.tiles.map((t) => t.id));
  if (!ids.length) return null;
  if (stacked) return ids.length === 1 ? leaf(ids[0]) : split('column', ids.map(leaf));
  // Rust never keeps an empty column, so indexes here are the tab's.
  const cols = columns.map((c, ci) =>
    c.tiles.length === 1
      ? leaf(c.tiles[0].id)
      : split('column', c.tiles.map((t) => leaf(t.id)), shares(sizes.rows[ci] ?? [], c.tiles.length)),
  );
  return cols.length === 1 ? cols[0] : split('row', cols, shares(sizes.columns, cols.length));
}

/** V107-N6b Shown shares with every column and tile at least `min(id)` px (plus `gap`) where
 * there's room: a 1:3 split from a Chatterino layout or a smaller window left a chat with its
 * header and message box and no lines. Too little room for all: each gets its part. Not saved. */
export function clampGrid(n: Node | null, box: Box, min: (id: string) => Box, gap = 0): Node | null {
  if (!n || box.w <= 0 || box.h <= 0) return n;
  // In `layout`'s own box: a gap larger, each panel a gap larger.
  const least = (id: string) => {
    const m = min(id);
    return { w: m.w + gap, h: m.h + gap };
  };
  const place = (x: Node, b: Box): Node => {
    if (!isSplit(x)) return x;
    const row = x.dir === 'row';
    const sizes = clampSizes(x.sizes, x.parts.map((p) => (row ? minOf(p, least).w : minOf(p, least).h)), row ? b.w : b.h);
    const parts = x.parts.map((p, i) => place(p, row ? { w: b.w * sizes[i], h: b.h } : { w: b.w, h: b.h * sizes[i] }));
    return { dir: x.dir, parts, sizes };
  };
  return place(n, { w: box.w + gap, h: box.h + gap });
}

/** Which of the tab's sizes a divider of `gridTree` changes: the columns', or the tiles'
 * of column `column`. */
export type GridLine = { kind: 'columns' } | { kind: 'rows'; column: number };

export function gridLine(d: Divider, columns: number): GridLine {
  if (d.dir === 'row') return { kind: 'columns' };
  // One column: its split is the whole tree.
  return { kind: 'rows', column: columns > 1 ? (d.path[0] ?? 0) : 0 };
}

/** New sizes after a divider was dragged: the split's new shares, kept averaging 1 as Rust
 * stores them. */
export function resized(sizes: GridSizes, line: GridLine, next: number[]): GridSizes {
  const mean = next.map((s) => s * next.length);
  if (line.kind === 'columns') return { columns: mean, rows: sizes.rows.map((r) => [...r]) };
  return { columns: [...sizes.columns], rows: sizes.rows.map((r, i) => (i === line.column ? mean : [...r])) };
}

/** The weight an offline-only column gets next to live ones, as a share of a default column. */
export const SLIM = 0.3;

/** The widest a slim card gets, in px: on a wide window it stays a card, not a black column. */
export const SLIM_MAX_PX = 360;

/** D6 Columns whose tiles are all offline shrink to a slim card, so live channels get the
 * room. Only columns still at their default width (1) shrink: one the user sized stays, and
 * so does everything when no column is live or all of them are offline. With the grid's
 * width (`width`, px) a slim column never gets wider than SLIM_MAX_PX; live ones take the rest. */
export function slimSizes(columns: readonly { size: number; tiles: readonly { id: string }[] }[], sizes: GridSizes, offline: (id: string) => boolean, width = 0): GridSizes {
  const off = columns.map((c) => c.tiles.length > 0 && c.tiles.every((t) => offline(t.id)));
  if (columns.length < 2 || off.every(Boolean) || !off.some(Boolean)) return sizes;
  const slim = off.map((o, i) => o && Math.abs(sizes.columns[i] - 1) < 0.001);
  let weight = SLIM;
  const n = slim.filter(Boolean).length;
  const others = sizes.columns.reduce((sum, s, i) => sum + (slim[i] ? 0 : s), 0);
  // The slim share of the width is w / (others + n w): solve for the cap.
  if (width > n * SLIM_MAX_PX && (SLIM * width) / (others + n * SLIM) > SLIM_MAX_PX) {
    weight = (SLIM_MAX_PX * others) / (width - n * SLIM_MAX_PX);
  }
  return { rows: sizes.rows, columns: sizes.columns.map((s, i) => (slim[i] ? weight : s)) };
}
