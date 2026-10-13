// V105-14 Reordering the miner list: where a dragged row lands.

/** `list` with the item at `from` moved before row `to`, or after it when `after`. */
export function moveTo<T>(list: readonly T[], from: number, to: number, after: boolean): T[] {
  let at = to + (after ? 1 : 0);
  if (from < at) at -= 1;
  const out = [...list];
  const [moved] = out.splice(from, 1);
  out.splice(at, 0, moved);
  return out;
}
