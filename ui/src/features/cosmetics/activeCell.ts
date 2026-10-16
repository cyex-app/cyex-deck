// V1051-03 Which grid cell has the keyboard: the one the user moved to, else the worn one,
// else None. Kept by id, so a filter that moves the cells doesn't move it.

/** The index of `moved` in `keys`; if it isn't there, the `worn` one; else the first (None). */
export function activeIndex(keys: readonly string[], moved: string | undefined, worn: string): number {
  const at = moved === undefined ? -1 : keys.indexOf(moved);
  if (at >= 0) return at;
  return Math.max(0, keys.indexOf(worn));
}
