// V108-N10 The header's messages-per-second meter: the number it shows and how long to wait
// between looks.
import { formatDecimal, formatNumber } from '../../lib/format.ts';

/** Looks at the count this often, ms. */
export const RATE_EVERY_MS = 2000;

/** 0.04 → "0/s", 3.24 → "3,2/s", 12.6 → "13/s", 1234 → "1.234/s". Never negative or NaN. */
export function rateText(perSec: number): string {
  const n = Number.isFinite(perSec) && perSec > 0 ? perSec : 0;
  if (n < 0.05) return '0/s';
  if (Math.round(n * 10) < 100) return `${formatDecimal(n, 1)}/s`;
  return `${formatNumber(Math.round(n))}/s`;
}

/** Whether the meter shows a busy chat (10 a second or more), for a stronger look. */
export function rateBusy(perSec: number): boolean {
  return Number.isFinite(perSec) && perSec >= 10;
}
