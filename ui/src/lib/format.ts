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

export function formatDate(d: Date): string {
  return `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())}`;
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
