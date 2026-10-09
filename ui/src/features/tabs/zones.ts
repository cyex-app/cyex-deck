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
