// V110-U64 Video stats: the dropped-frames text, and which tiles (one per tab entry, by its
// uid) have the overlay on. Kept in the window's own storage, so a tab you come back to
// shows it as you left it. Off by default.
import { formatDecimal, formatNumber } from '../../lib/format.ts';

/** Share of frames dropped, in percent. 0 before any frame. */
export function dropPercent(dropped: number, total: number): number {
  return total > 0 ? Math.min(100, (Math.max(0, dropped) / total) * 100) : 0;
}

/** "0,4 % (12)". Empty when the browser gives no playback quality. */
export function dropText(q: { dropped: number; total: number } | null): string {
  if (!q) return '';
  return `${formatDecimal(dropPercent(q.dropped, q.total))} % (${formatNumber(q.dropped)})`;
}

const KEY = 'cyex.videoStats';
const MAX = 200;

/** The tile uids with the overlay on, oldest first. */
export function withStats(list: readonly string[], id: string, on: boolean): string[] {
  const rest = list.filter((x) => x !== id);
  return on ? [...rest, id].slice(-MAX) : rest;
}

function load(): string[] {
  try {
    const v: unknown = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

export function statsWanted(id: string): boolean {
  return load().includes(id);
}

export function rememberStats(id: string, on: boolean): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(withStats(load(), id, on)));
  } catch {
    // Storage is blocked: the choice just lasts until the player goes.
  }
}
