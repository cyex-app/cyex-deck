// NumberField's rules, kept apart so they can be tested.

export interface NumberRange {
  min: number;
  max: number;
  /** Arrow-key step and rounding grid (default 1). */
  step?: number;
}

/** V110-F84 Plain decimals only: `0x10`, `1e3` and `Infinity` are not numbers typed into a box. */
function plain(t: string): number | null {
  if (!/^[+-]?(\d+\.?\d*|\.\d+)$/.test(t)) return null;
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
}

/** Inside the range and on the step grid. */
export function clampNumber(value: number, { min, max, step = 1 }: NumberRange): number {
  // V110-F89 A step of 0 or less is no grid (dividing by it gave NaN).
  const snapped = step > 0 ? Math.round((value - min) / step) * step + min : value;
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
  const n = plain(t);
  return n !== null ? clampNumber(n, range) : null;
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

/** What was typed as a plain number, not clamped; null when it isn't one. Same reading as parseNumber. */
export function readNumber(text: string): number | null {
  let t = text.trim().replace(/\s/g, '');
  if (!t) return null;
  if (t.includes(',')) t = t.replace(/\./g, '').replace(',', '.');
  else if (/^-?\d{1,3}(\.\d{3})+$/.test(t)) t = t.replace(/\./g, '');
  const n = plain(t);
  return n;
}

/** "Max 5.000" / "Min 100" when the typed number is outside the range; null when it fits or isn't a number.
 *  `fmt` groups the digits. */
export function rangeNote(text: string, { min, max }: NumberRange, fmt: (n: number) => string): string | null {
  const n = readNumber(text);
  if (n === null) return null;
  if (n > max) return `Max ${fmt(max)}`;
  if (n < min) return `Min ${fmt(min)}`;
  return null;
}
