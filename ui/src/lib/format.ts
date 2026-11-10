// Date, time, number and duration formatting. Dates are yyyy/mm/dd (3.1), times 24-hour (3.2)
// unless chat timestamps are set to 12-hour (2.10), numbers grouped with dots (3.13).

const pad = (n: number) => String(n).padStart(2, '0');

/** V110-F84 Not a number, infinite, or so big that String() turns to "1e+21": shown as a dash. */
const unreadable = (n: number) => !Number.isFinite(n) || Math.abs(n) >= 1e21;

/** 1234567 → "1.234.567". Never the OS locale's separator (3.13). */
export function formatNumber(n: number): string {
  if (unreadable(n)) return '-';
  const digits = String(Math.trunc(Math.abs(n))).replace(/\B(?=(\d{3})+$)/g, '.');
  return n <= -1 ? `-${digits}` : digits;
}

const SHORT: [suffix: string, size: number][] = [
  ['B', 1e9],
  ['M', 1e6],
  ['K', 1e3],
];

/** 999 → "999", 1250 → "1,2K", 2_000_000 → "2M". Rounds down; comma decimal so it can't read as a grouped number. */
export function formatShort(n: number): string {
  if (unreadable(n)) return '-';
  const abs = Math.abs(n);
  const unit = SHORT.find(([, size]) => abs >= size);
  if (!unit) return formatNumber(n);
  const tenths = Math.floor((abs * 10) / unit[1]);
  const text = `${tenths % 10 ? (tenths / 10).toFixed(1).replace('.', ',') : tenths / 10}${unit[0]}`;
  return n < 0 ? `-${text}` : text;
}

/** 1.5 → "1,5", fixed to `digits` places. The decimal mark is a comma, like formatShort (3.13). */
export function formatDecimal(n: number, digits = 1): string {
  if (unreadable(n)) return '-'; // V110-F89
  return n.toFixed(digits).replace('.', ',');
}

/** A speed or factor: 0.75 → "0,75×", 2 → "2×". As many decimals as it has, comma decimal (3.13). */
export function formatFactor(n: number): string {
  if (unreadable(n)) return '-'; // V110-F89
  return `${String(n).replace('.', ',')}×`;
}

/** 39_845_888 → "38 MB", 1_300_000_000 → "1,2 GB". Binary units, comma decimal, dots for thousands (3.13). */
export function formatBytes(bytes: number): string {
  if (unreadable(bytes)) return '-';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = Math.max(0, bytes);
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  if (i === 0) return `${formatNumber(value)} B`;
  let rounded = value >= 100 ? Math.round(value) : Math.round(value * 10) / 10;
  // V110-F84 Rounding can land on the next unit's edge (1023,96 KB → 1.024 KB): count it in that unit.
  if (rounded >= 1024 && i < units.length - 1) {
    i++;
    rounded = Math.round((rounded / 1024) * 10) / 10;
  }
  const text = Number.isInteger(rounded) ? formatNumber(rounded) : formatDecimal(rounded, 1);
  return `${text} ${units[i]}`;
}

export function formatDate(d: Date): string {
  if (Number.isNaN(d.getTime())) return '-'; // V110-F89
  return `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())}`;
}

const AGO_UNITS: [name: string, secs: number][] = [
  ['year', 31_536_000],
  ['month', 2_592_000],
  ['day', 86_400],
  ['hour', 3_600],
  ['minute', 60],
];

/** How long ago, short: "now", "5m", "3h", "2d", then the date. `words`: "3 days ago",
 *  "2 months ago", "just now" (V107-R40, hovers and lists). The one relative time. */
export function ago(ms: number, now = Date.now(), words = false): string {
  if (unreadable(ms) || unreadable(now)) return '-'; // V110-F89
  const s = Math.max(0, Math.floor((now - ms) / 1000));
  if (words) {
    for (const [name, size] of AGO_UNITS) {
      const n = name === 'month' ? Math.min(11, Math.floor(s / size)) : Math.floor(s / size);
      // V110-F84 360 to 364 days are not "12 months": a year is 365.
      if (n >= 1) return `${n} ${name}${n === 1 ? '' : 's'} ago`;
    }
    return 'just now';
  }
  if (s < 60) return 'now';
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  if (s < 86_400) return `${Math.floor(s / 3600)}h`;
  if (s < 7 * 86_400) return `${Math.floor(s / 86_400)}d`;
  return formatDate(new Date(ms));
}

export interface TimeOptions {
  seconds?: boolean;
  clock?: 'h24' | 'h12';
}

/** V110-U53 The Time format setting, for times shown without their own options. */
let appClock: 'h24' | 'h12' = 'h24';
let clockOf: () => 'h24' | 'h12' = () => appClock;
/** Fixes the clock (tests, or before the settings store exists). */
export function setAppClock(clock: 'h24' | 'h12'): void {
  appClock = clock;
  clockOf = () => appClock;
}
/** V110-F20 The settings store hands over a reader of its own state, so a template or `$derived`
 * calling `formatTime(d)` follows the setting (this file stays free of runes for the node tests). */
export function useClockSource(get: () => 'h24' | 'h12'): void {
  clockOf = get;
}

export function formatTime(d: Date, { seconds = false, clock = clockOf() }: TimeOptions = {}): string {
  if (Number.isNaN(d.getTime())) return '-'; // V110-F89
  const s = seconds ? `:${pad(d.getSeconds())}` : '';
  if (clock !== 'h12') return `${pad(d.getHours())}:${pad(d.getMinutes())}${s}`;
  const h = d.getHours() % 12 || 12;
  return `${h}:${pad(d.getMinutes())}${s} ${d.getHours() < 12 ? 'am' : 'pm'}`;
}

/** V110-C65 The full time of a line, for a hover: 2026/11/08 14:03:09 in the Time format, seconds always. */
export function formatStamp(ms: number): string {
  if (unreadable(ms) || Number.isNaN(new Date(ms).getTime())) return '-'; // V110-F89
  const d = new Date(ms);
  return `${formatDate(d)} ${formatTime(d, { seconds: true })}`;
}

const UNITS: [suffix: string, seconds: number][] = [
  ['w', 604_800],
  ['d', 86_400],
  ['h', 3_600],
  ['m', 60],
  ['s', 1],
];

/** 90 → "1m 30s", 3600 → "1h". */
export function formatDuration(totalSeconds: number): string {
  if (unreadable(totalSeconds)) return '-'; // V110-F89
  let rest = Math.max(0, Math.round(totalSeconds));
  const parts: string[] = [];
  for (const [suffix, size] of UNITS) {
    const n = Math.floor(rest / size);
    if (n > 0) parts.push(`${n}${suffix}`);
    rest -= n * size;
  }
  return parts.slice(0, 2).join(' ') || '0s';
}

/** "30s", "5m", "1h 30m", "2d", or a bare number of seconds. Null if unreadable. */
export function parseDuration(text: string): number | null {
  const input = text.trim().toLowerCase();
  if (!input) return null;
  if (/^\d+$/.test(input)) return Number(input);
  const re = /(\d+)\s*([wdhms])/g;
  let total = 0;
  let consumed = '';
  for (const match of input.matchAll(re)) {
    const size = UNITS.find(([suffix]) => suffix === match[2])![1];
    total += Number(match[1]) * size;
    consumed += match[0];
  }
  return consumed.replace(/\s/g, '') === input.replace(/\s/g, '') && consumed ? total : null; // V110-F84 "0s" is 0 like "0" is; callers range-check
}

const MONTH = 2_592_000;

/** Compact length: 30 -> "30s", 90 -> "90s", 300 -> "5m", 3600 -> "1h", 5400 -> "90m", 86400 -> "1d", 604800 -> "1w", 7776000 -> "3mo" (30-day months). Up to 3 minutes stays in seconds; a decimal second keeps its comma. */
export function formatSecs(secs: number): string {
  if (unreadable(secs)) return '-';
  const n = Math.max(0, secs);
  if (!Number.isInteger(n)) return `${formatDecimal(n)}s`;
  if (n <= 180) return `${n}s`;
  if (n >= MONTH && n % MONTH === 0) return `${n / MONTH}mo`;
  if (n >= 604_800 && n % 604_800 === 0) return `${n / 604_800}w`;
  if (n >= 86_400 && n % 86_400 === 0) return `${n / 86_400}d`;
  if (n % 3600 === 0) return `${n / 3600}h`;
  if (n % 60 === 0) return `${n / 60}m`;
  return formatDuration(n);
}
