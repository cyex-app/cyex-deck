// V111-05 Where a row dragged in a DragList lands, from the rows' boxes alone. Plain
// functions so they can be tested.
//
// The cause of the glitchy drags: the drop spot came from the element under the pointer,
// so a pointer just past the last row, above the first, beside the list or over a gap
// between rows found no row and the drop was thrown away. This works from every row's box,
// so anywhere near the list lands in the closest slot.

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Pixels around the list that still count as on it. */
export const SLOT_MARGIN = 48;

/** The slot at (x, y) among `rows` (boxes in list order): before or after row `index`.
 *  Rows that share a line (a wrapping list) split by their horizontal middles, lines by
 *  their vertical ones; a gap goes to the nearest line. `null` when the list is empty or
 *  the point is more than `margin` away from every row. */
export function rowSlotAt(rows: readonly Box[], x: number, y: number, margin = SLOT_MARGIN): { index: number; after: boolean } | null {
  if (!rows.length) return null;
  const left = Math.min(...rows.map((r) => r.left));
  const right = Math.max(...rows.map((r) => r.left + r.width));
  const top = Math.min(...rows.map((r) => r.top));
  const bottom = Math.max(...rows.map((r) => r.top + r.height));
  if (x < left - margin || x > right + margin || y < top - margin || y > bottom + margin) return null;

  // Lines: runs of rows whose boxes overlap vertically (one row each in a column).
  const lines: number[][] = [];
  rows.forEach((r, i) => {
    const line = lines[lines.length - 1];
    const prev = line && rows[line[0]];
    if (prev && r.top < prev.top + prev.height && prev.top < r.top + r.height) line.push(i);
    else lines.push([i]);
  });

  if (lines.every((l) => l.length === 1)) {
    // A column: before the first row whose middle is below the pointer, else after the last.
    const i = rows.findIndex((r) => y < r.top + r.height / 2);
    return i < 0 ? { index: rows.length - 1, after: true } : { index: i, after: false };
  }

  // Wrapped: the line the pointer is on, or the nearest one; then by middles across it.
  const dist = (l: number[]) => {
    const t = rows[l[0]].top;
    const b = t + rows[l[0]].height;
    return y < t ? t - y : y > b ? y - b : 0;
  };
  const line = lines.reduce((a, b) => (dist(b) < dist(a) ? b : a));
  const i = line.find((j) => x < rows[j].left + rows[j].width / 2);
  return i === undefined ? { index: line[line.length - 1], after: true } : { index: i, after: false };
}

/** Whether dropping the row at `from` before (or `after`) row `to` leaves the list as it
 *  is: its own slot, or the edge it shares with a neighbour. */
export function sameSlot(from: number, to: number, after: boolean): boolean {
  const at = to + (after ? 1 : 0);
  return at === from || at === from + 1;
}
