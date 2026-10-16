// V1051-01 Keyboard moves for Select's list, kept apart so they can be tested.

export interface Choice {
  value: string;
  label: string;
}

/** The next index for a key, or null when the key isn't ours. */
export function moveFor(key: string, at: number, count: number): number | null {
  if (!count) return null;
  switch (key) {
    case 'ArrowDown':
      return Math.min(at + 1, count - 1);
    case 'ArrowUp':
      return Math.max(at - 1, 0);
    case 'Home':
      return 0;
    case 'End':
      return count - 1;
    case 'PageDown':
      return Math.min(at + 8, count - 1);
    case 'PageUp':
      return Math.max(at - 8, 0);
    default:
      return null;
  }
}

/** Type-ahead: the next choice (after `at`, wrapping) whose label starts with `typed`. */
export function matchTyped(choices: readonly Choice[], typed: string, at: number): number {
  const t = typed.toLowerCase();
  if (!t) return -1;
  const n = choices.length;
  // One letter (or the same one again and again) cycles through the choices that start with it.
  const cycle = [...t].every((ch) => ch === t[0]);
  const from = cycle ? at + 1 : at;
  const key = cycle ? t[0] : t;
  for (let i = 0; i < n; i++) {
    const j = (from + i + n) % n;
    if (choices[j].label.toLowerCase().startsWith(key)) return j;
  }
  return -1;
}
