// V111-23 The bar's latency figure: "2,1 s", and when it is high enough to warn about.
import { formatDecimal } from '../../lib/format.ts';

/** Seconds from which the figure turns to the warning tone. */
export const LATENCY_WARN = 10;

/** "2,1 s". Empty until there is a reading (no jump on a layout that reserves its width). */
export function latencyText(s: number | null): string {
  return s === null || !Number.isFinite(s) ? '' : `${formatDecimal(s)} s`;
}

export const latencyHigh = (s: number | null): boolean => s !== null && Number.isFinite(s) && s > LATENCY_WARN;
