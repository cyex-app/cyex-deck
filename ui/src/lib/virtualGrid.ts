// V1051-03 Math for a windowed grid of fixed-size cells: which to mount, where the
// keyboard goes, how far to scroll. Pure, so it's tested without a DOM.

export interface Geometry {
  columns: number;
  rowHeight: number;
  rows: number;
}

export function geometry(width: number, count: number, cellW: number, cellH: number, gap: number): Geometry {
  const columns = Math.max(1, Math.floor((width + gap) / (cellW + gap)));
  return { columns, rowHeight: cellH + gap, rows: Math.ceil(count / columns) };
}

/** First and last index (inclusive) to mount, with `overscan` rows above and below. */
export function range(
  g: Geometry,
  count: number,
  scrollTop: number,
  viewH: number,
  overscan = 2,
): { first: number; last: number } {
  if (count <= 0 || viewH <= 0 || g.rows <= 0) return { first: 0, last: -1 };
  const top = Math.max(0, Math.floor(scrollTop / g.rowHeight) - overscan);
  const bottom = Math.min(g.rows - 1, Math.ceil((scrollTop + viewH) / g.rowHeight) - 1 + overscan);
  if (top > bottom) return { first: 0, last: -1 };
  return { first: top * g.columns, last: Math.min(count - 1, (bottom + 1) * g.columns - 1) };
}

/** Keyboard: ArrowLeft/Right/Up/Down, Home, End, PageUp, PageDown. Returns the new index, clamped. */
export function step(index: number, key: string, g: Geometry, count: number, viewH: number): number {
  if (count <= 0) return 0;
  const last = count - 1;
  const page = Math.max(1, Math.floor(viewH / g.rowHeight)) * g.columns;
  const row = (i: number) => Math.floor(i / g.columns);
  let next = index;
  switch (key) {
    case 'ArrowLeft':
      next = index - 1;
      break;
    case 'ArrowRight':
      next = index + 1;
      break;
    case 'ArrowUp':
      next = index - g.columns;
      break;
    case 'ArrowDown':
      // A short last row: land on its end, but never leave the last row.
      next = index + g.columns;
      if (next > last) next = row(index) < row(last) ? last : index;
      break;
    case 'Home':
      next = 0;
      break;
    case 'End':
      next = last;
      break;
    case 'PageUp':
      next = index - page;
      break;
    case 'PageDown':
      next = index + page;
      break;
  }
  return Math.min(last, Math.max(0, next));
}

/** The scrollTop that brings `index` fully into view, or null if it already is. */
export function reveal(index: number, g: Geometry, scrollTop: number, viewH: number): number | null {
  const top = Math.floor(index / g.columns) * g.rowHeight;
  const bottom = top + g.rowHeight;
  if (top < scrollTop) return top;
  if (bottom > scrollTop + viewH) return Math.max(0, bottom - viewH);
  return null;
}

/** The width of a scroller that ends right after the last column (bar not counted). */
export const boxWidth = (columns: number, cellW: number, gap: number): number => columns * cellW + (columns - 1) * gap;
