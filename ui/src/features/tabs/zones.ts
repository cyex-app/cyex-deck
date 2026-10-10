// Where dragged tabs and tiles land (U3, U5). Plain functions so they can be tested.

import type { Zone } from '../../ipc/bindings/Zone';

interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** U3 A tile dropped on another: the middle swaps them, an edge puts it on that side. */
export function zoneAt(r: Rect, x: number, y: number): Zone {
  const fx = (x - r.left) / r.width;
  const fy = (y - r.top) / r.height;
  const dx = Math.min(fx, 1 - fx);
  const dy = Math.min(fy, 1 - fy);
  if (dx > 0.3 && dy > 0.3) return 'center';
  if (dx < dy) return fx < 0.5 ? 'left' : 'right';
  return fy < 0.5 ? 'top' : 'bottom';
}

interface Grouped {
  group: string | null;
}

/** U5 The group a tab joins when dropped between `prev` and `next`. Inside a group's
 * run it joins that group; right at the edge of its own group it stays in it. */
export function groupAt(prev: Grouped | undefined, next: Grouped | undefined, own: string | null): string | null {
  if (prev?.group && prev.group === next?.group) return prev.group;
  if (own && (prev?.group === own || next?.group === own)) return own;
  return null;
}

/** V101-38 A tab dropped in a gap of a wrapped bar, or past a row's last tab: the slot
 * next to the closest tab in that row. `cells` are the tabs in bar order. `null` below
 * every row, which means the end of the bar. */
export function rowSlot(cells: { id: string; rect: Rect }[], x: number, y: number): { id: string; after: boolean } | null {
  const row = cells.filter((c) => y >= c.rect.top && y < c.rect.top + c.rect.height);
  if (!row.length) return null;
  const next = row.find((c) => x < c.rect.left + c.rect.width / 2);
  return next ? { id: next.id, after: false } : { id: row[row.length - 1].id, after: true };
}
