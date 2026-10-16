// NumberField's rules, kept apart so they can be tested.

export interface NumberRange {
  min: number;
  max: number;
  /** Arrow-key step and rounding grid (default 1). */
  step?: number;
}

/** Inside the range and on the step grid. */
export function clampNumber(value: number, { min, max, step = 1 }: NumberRange): number {
  const snapped = Math.round((value - min) / step) * step + min;
  // Float noise from steps like 0.1.
  const tidy = Number(snapped.toFixed(6));
  return Math.min(Math.max(tidy, min), max);
}

/** What was typed, as a number in the range; null when it isn't a number (keep the old one).
 *  A comma works as the decimal mark, dots group thousands (1.000 is a thousand). */
export function parseNumber(text: string, range: NumberRange): number | null {
  let t = text.trim().replace(/\s/g, '');
  if (!t) return null;
  if (t.includes(',')) t = t.replace(/\./g, '').replace(',', '.');
  else if (/^-?\d{1,3}(\.\d{3})+$/.test(t)) t = t.replace(/\./g, '');
  const n = Number(t);
  return Number.isFinite(n) ? clampNumber(n, range) : null;
}

/** Up/Down move one step, PageUp/PageDown or Shift ten, Home/End to the ends; null for other keys. */
export function stepFor(key: string, shift: boolean, value: number, range: NumberRange): number | null {
  const step = range.step ?? 1;
  const by = shift ? step * 10 : step;
  switch (key) {
    case 'ArrowUp':
      return clampNumber(value + by, range);
    case 'ArrowDown':
      return clampNumber(value - by, range);
    case 'PageUp':
      return clampNumber(value + step * 10, range);
    case 'PageDown':
      return clampNumber(value - step * 10, range);
    case 'Home':
      return range.min;
    case 'End':
      return range.max;
    default:
      return null;
  }
}
