// 1.35, X4 Pure helpers for the stream manager: activity filters, the viewer graph and the
// ad countdown. Kept free of Svelte so they run under the UI tests.

import type { ActivityFilters } from '../../ipc/bindings/ActivityFilters';
import type { SmActivity } from '../../ipc/bindings/SmActivity';

/** Feed kinds (Rust's `Item.kind`), their filter key and chip label. */
export const ACTIVITY_KINDS: { kind: string; key: keyof ActivityFilters; label: string }[] = [
  { kind: 'follow', key: 'follows', label: 'Follows' },
  { kind: 'sub', key: 'subs', label: 'Subs' },
  { kind: 'gift', key: 'gifts', label: 'Gifts' },
  { kind: 'cheer', key: 'cheers', label: 'Cheers' },
  { kind: 'raid', key: 'raids', label: 'Raids' },
  { kind: 'redemption', key: 'redemptions', label: 'Rewards' },
  { kind: 'streak', key: 'streaks', label: 'Streaks' },
];

/** Newest first, only the kinds switched on. Unknown kinds always show. */
export function visibleActivity(items: SmActivity[], filters: ActivityFilters): SmActivity[] {
  const keyOf = new Map(ACTIVITY_KINDS.map((k) => [k.kind, k.key]));
  return items
    .filter((i) => {
      const key = keyOf.get(i.kind);
      return key ? filters[key] : true;
    })
    .slice()
    .reverse();
}

/** Appends and drops the oldest past `max`. Returns a new array. */
export function capPush<T>(list: T[], item: T, max: number): T[] {
  const next = [...list, item];
  return next.length > max ? next.slice(next.length - max) : next;
}

export interface Graph {
  /** SVG polyline points, `x,y x,y`. Empty under two samples. */
  line: string;
  /** Closed path under the line, for the fill. */
  area: string;
  min: number;
  max: number;
}

/** The viewer graph scaled into `w`×`h`, oldest left. `series` is [unix seconds, viewers]. */
export function graph(series: [number, number][], w: number, h: number): Graph {
  if (series.length < 2) return { line: '', area: '', min: 0, max: 0 };
  const t0 = series[0][0];
  const span = Math.max(1, series[series.length - 1][0] - t0);
  const values = series.map(([, v]) => v);
  const min = Math.min(...values);
  const max = Math.max(...values);
  // A flat line sits in the middle instead of on the floor.
  const range = max - min || 1;
  const pad = 2;
  const pts = series.map(([t, v]) => {
    const x = ((t - t0) / span) * w;
    const y = max === min ? h / 2 : pad + (1 - (v - min) / range) * (h - pad * 2);
    return `${round(x)},${round(y)}`;
  });
  const line = pts.join(' ');
  return { line, area: `M0,${h} L${pts.join(' L')} L${w},${h} Z`, min, max };
}

const round = (n: number) => Math.round(n * 10) / 10;

/** Seconds until the next ad, or null when none is scheduled. Never below 0. */
export function adIn(nextAdAt: string, now = Date.now()): number | null {
  const t = Date.parse(nextAdAt);
  if (!nextAdAt || Number.isNaN(t) || t <= 0) return null;
  return Math.max(0, Math.round((t - now) / 1000));
}

/** Prediction outcomes' share of the points, in whole percent. */
export function shares(points: number[]): number[] {
  const total = points.reduce((a, b) => a + b, 0);
  return points.map((p) => (total ? Math.round((p * 100) / total) : 0));
}
