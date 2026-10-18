// 1.40/1.43 Arrow keys across picker cells by where they sit on screen, so it works for
// wrapped rows (emotes) and for columns of mixed heights (GIFs) alike.

export type Dir = 'left' | 'right' | 'up' | 'down';

interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export const ARROWS: Record<string, Dir> = {
  ArrowLeft: 'left',
  ArrowRight: 'right',
  ArrowUp: 'up',
  ArrowDown: 'down',
};

/** The index of the nearest box from `boxes[from]` in `dir`, or -1 at the edge. Sideways
 * stays in the same row band; up and down prefer the same column, then the nearest. */
export function nearest(boxes: Box[], from: number, dir: Dir): number {
  const a = boxes[from];
  if (!a) return -1;
  const cx = (a.left + a.right) / 2;
  const cy = (a.top + a.bottom) / 2;
  let best = -1;
  let score = Infinity;
  boxes.forEach((b, i) => {
    if (i === from) return;
    const bx = (b.left + b.right) / 2;
    const by = (b.top + b.bottom) / 2;
    let main: number;
    let cross: number;
    if (dir === 'left' || dir === 'right') {
      if (b.bottom <= a.top || b.top >= a.bottom) return;
      main = dir === 'right' ? bx - cx : cx - bx;
      cross = Math.abs(by - cy);
    } else {
      main = dir === 'down' ? by - cy : cy - by;
      const overlap = b.right > a.left && b.left < a.right;
      cross = overlap ? Math.abs(bx - cx) : Math.abs(bx - cx) * 4;
    }
    if (main <= 1) return;
    const s = main + cross;
    if (s < score) {
      score = s;
      best = i;
    }
  });
  return best;
}

/** Moves focus from `from` to the nearest of `cells`; false at the edge. */
export function moveFocus(cells: HTMLElement[], from: HTMLElement, dir: Dir): boolean {
  const i = cells.indexOf(from);
  if (i < 0) return false;
  const next = nearest(
    cells.map((c) => c.getBoundingClientRect()),
    i,
    dir,
  );
  if (next < 0) return false;
  cells[next].focus();
  return true;
}
