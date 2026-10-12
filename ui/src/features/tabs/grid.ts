// F13 (v1.04 batch 2b) A tab's columns of tiles (U3) as one panel tree, so `layout` in
// `lib/panels.ts` places every tile absolutely, the way panels are placed since V103-05.
// The tiles stay in one list in a fixed order in the page; moving one only changes its box,
// so no stream in the tab restarts.

import { type Divider, leaf, type Node, shares, split } from '../../lib/panels.ts';

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
