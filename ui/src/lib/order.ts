// Moving one item of a list before or after another: the miner list (V105-14) and mod mode's
// buttons (V105-15), dragged with DragList.

/** `list` with the item at `from` moved before row `to`, or after it when `after`. */
export function moveTo<T>(list: readonly T[], from: number, to: number, after: boolean): T[] {
  let at = to + (after ? 1 : 0);
  if (from < at) at -= 1;
  const out = [...list];
  const [moved] = out.splice(from, 1);
  out.splice(at, 0, moved);
  return out;
}

/** `list` with the item keyed `from` moved before the one keyed `to`, or after it when
 *  `after`. `null` when a key is gone or nothing moves. */
export function moveKey<T>(list: readonly T[], key: (t: T) => string, from: string, to: string, after: boolean): T[] | null {
  const i = list.findIndex((t) => key(t) === from);
  const j = list.findIndex((t) => key(t) === to);
  if (i < 0 || j < 0) return null;
  const out = moveTo(list, i, j, after);
  return out.every((t, n) => t === list[n]) ? null : out;
}

/** What Alt+ArrowUp / Alt+ArrowDown does to the row keyed `id`: the new list and the row's
 *  1-based place in it, or `null` at either end (or for a key that's gone). */
export function nudge<T>(list: readonly T[], key: (t: T) => string, id: string, dir: -1 | 1): { items: T[]; place: number } | null {
  const i = list.findIndex((t) => key(t) === id);
  const j = i + dir;
  if (i < 0 || j < 0 || j >= list.length) return null;
  const items = moveKey(list, key, id, key(list[j]), dir > 0);
  return items && { items, place: j + 1 };
}

/** V106-10 Which way a key moves a row: Alt+ArrowUp -1, Alt+ArrowDown 1, anything else null. */
export function nudgeDir(e: { altKey: boolean; key: string }): -1 | 1 | null {
  if (!e.altKey) return null;
  return e.key === 'ArrowUp' ? -1 : e.key === 'ArrowDown' ? 1 : null;
}
