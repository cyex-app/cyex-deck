// 1.40/1.43 Arrow keys across picker cells by where they sit on screen, so it works for
// wrapped rows (emotes) and for columns of mixed heights (GIFs) alike. Plus the GIF split.

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
  // fg-5 Nearest, so the list's scroll-padding keeps the focus ring whole at its edge.
  cells[next].focus({ preventScroll: true });
  cells[next].scrollIntoView({ block: 'nearest', inline: 'nearest' });
  return true;
}

/** 1.43 Splits items over `cols` columns, each to the shortest so far (height by `ratio`,
 * height over width, plus `gap` in the same units). The split of a list's start never
 * changes when more items come, so tiles don't jump on the next page. */
export function columns<T>(items: T[], ratio: (item: T) => number, cols = 2, gap = 0): T[][] {
  const out: T[][] = Array.from({ length: cols }, () => []);
  const h = new Array<number>(cols).fill(0);
  for (const it of items) {
    let c = 0;
    for (let i = 1; i < cols; i++) if (h[i] < h[c] - 1e-9) c = i;
    out[c].push(it);
    const r = ratio(it);
    h[c] += (Number.isFinite(r) && r > 0 ? r : 1) + gap;
  }
  return out;
}
