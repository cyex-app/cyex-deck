// V1051-01 Keyboard moves for Select's list, kept apart so they can be tested.

export interface Choice {
  value: string;
  label: string;
  /** A 20 px round picture beside the label. `null` shows the first letter; leave out for none. */
  picture?: string | null;
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

/**
 * U20 The index a roving radio group moves to for a key, or null when the key isn't ours.
 * Left/Right work like Up/Down and wrap around; Home and End jump.
 */
export function radioKey(e: Pick<KeyboardEvent, 'key'>, at: number, n: number): number | null {
  if (!n) return null;
  switch (e.key) {
    case 'ArrowRight':
    case 'ArrowDown':
      return (at + 1) % n;
    case 'ArrowLeft':
    case 'ArrowUp':
      return (at - 1 + n) % n;
    default:
      return moveFor(e.key, at, n);
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

/** Choices from a list whose values are numbers (or typed strings): Select's values are strings. */
export function choicesOf(list: readonly { value: string | number; label: string }[]): Choice[] {
  return list.map((o) => ({ value: String(o.value), label: o.label }));
}
