// Date, time, number and duration formatting. Dates are yyyy/mm/dd (3.1), times 24-hour (3.2)
// unless chat timestamps are set to 12-hour (2.10), numbers grouped with dots (3.13).

const pad = (n: number) => String(n).padStart(2, '0');

/** 1234567 → "1.234.567". Never the OS locale's separator (3.13). */
export function formatNumber(n: number): string {
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
  const abs = Math.abs(n);
  const unit = SHORT.find(([, size]) => abs >= size);
  if (!unit) return formatNumber(n);
  const tenths = Math.floor((abs * 10) / unit[1]);
  const text = `${tenths % 10 ? (tenths / 10).toFixed(1).replace('.', ',') : tenths / 10}${unit[0]}`;
  return n < 0 ? `-${text}` : text;
}

/** 1.5 → "1,5", fixed to `digits` places. The decimal mark is a comma, like formatShort (3.13). */
export function formatDecimal(n: number, digits = 1): string {
  return n.toFixed(digits).replace('.', ',');
}

/** 39_845_888 → "38 MB", 1_300_000_000 → "1,2 GB". Binary units, comma decimal, dots for thousands (3.13). */
export function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = Math.max(0, bytes);
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  if (i === 0) return `${formatNumber(value)} B`;
  const rounded = value >= 100 ? Math.round(value) : Math.round(value * 10) / 10;
  const text = Number.isInteger(rounded) ? formatNumber(rounded) : formatDecimal(rounded, 1);
  return `${text} ${units[i]}`;
}

export function formatDate(d: Date): string {
  return `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())}`;
}

/** How long ago, short: "now", "5m", "3h", "2d", then the date. */
export function ago(ms: number, now = Date.now()): string {
  const s = Math.max(0, Math.floor((now - ms) / 1000));
  if (s < 60) return 'now';
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  if (s < 86_400) return `${Math.floor(s / 3600)}h`;
  if (s < 7 * 86_400) return `${Math.floor(s / 86_400)}d`;
  return formatDate(new Date(ms));
}

export interface TimeOptions {
  seconds?: boolean;
  hour12?: boolean;
}

export function formatTime(d: Date, { seconds = false, hour12 = false }: TimeOptions = {}): string {
  const s = seconds ? `:${pad(d.getSeconds())}` : '';
  if (!hour12) return `${pad(d.getHours())}:${pad(d.getMinutes())}${s}`;
  const h = d.getHours() % 12 || 12;
  return `${h}:${pad(d.getMinutes())}${s} ${d.getHours() < 12 ? 'am' : 'pm'}`;
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
  return consumed.replace(/\s/g, '') === input.replace(/\s/g, '') && total > 0 ? total : null;
}

/** Compact length: 30 -> "30s", 90 -> "90s", 300 -> "5m", 3600 -> "1h", 5400 -> "90m". Up to 3 minutes stays in seconds; a decimal second keeps its comma. */
export function formatSecs(secs: number): string {
  const n = Math.max(0, secs);
  if (!Number.isInteger(n)) return `${formatDecimal(n)}s`;
  if (n <= 180) return `${n}s`;
  if (n % 3600 === 0) return `${n / 3600}h`;
  if (n % 60 === 0) return `${n / 60}m`;
  return formatDuration(n);
}
