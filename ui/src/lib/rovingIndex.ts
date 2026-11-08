// V110-F32 Roving tab stop for a vertical list (the Settings nav): where Up, Down, Home and
// End land. No wrap at the ends, so the list's edges stay where they are.

/** The index `key` moves to from `i` in a list of `n`, or null when the key isn't one of ours. */
export function rovingIndex(key: string, i: number, n: number): number | null {
  if (n < 1) return null;
  switch (key) {
    case 'ArrowDown':
      return Math.min(i + 1, n - 1);
    case 'ArrowUp':
      return Math.max(i - 1, 0);
    case 'Home':
      return 0;
    case 'End':
      return n - 1;
    default:
      return null;
  }
}
