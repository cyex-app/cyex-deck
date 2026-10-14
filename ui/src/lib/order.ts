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
