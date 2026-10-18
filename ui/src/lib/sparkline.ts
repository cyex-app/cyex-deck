// 1.35, X4, V106-18 Viewer graph maths: a series scaled into a box, and which point sits
// under a pointer. Kept free of Svelte so they run under the UI tests.

export interface Graph {
  /** SVG polyline points, `x,y x,y`. Empty under two samples. */
  line: string;
  /** Closed path under the line, for the fill. */
  area: string;
  min: number;
  max: number;
  /** Where the first sample sits; with an axis start (`from`) the space left of it is "before". */
  x0: number;
}

const round = (n: number) => Math.round(n * 10) / 10;

/** Largest share of the width the "before" band takes (V106-18). */
export const BAND_SHARE = 0.12;

/**
 * Time to x. Without `from` the samples fill `w`. With `from` before the first sample, a
 * "before" band takes a fixed small share (at most BAND_SHARE, less for a small gap) and the
 * samples use the rest, scaled over the watched time only.
 */
function axis(series: [number, number][], w: number, from?: number) {
  const first = series[0][0];
  const last = series[series.length - 1][0];
  const gap = from !== undefined ? Math.max(0, first - from) : 0;
  const total = gap + (last - first);
  const band = gap > 0 ? Math.min(BAND_SHARE, gap / total) * w : 0;
  const span = Math.max(1, last - first);
  return (t: number) => band + ((t - first) / span) * (w - band);
}

/**
 * The value range the box shows, [low, high]. Tight by default (min..max; 0 when the low end
 * is small next to the peak). `padded` (c-6) leaves at least 10 % of the peak above and below
 * the data, never under 0, so a small change reads small and the lowest stretch keeps a fill.
 */
export function domain(values: number[], padded = false): [number, number] {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const small = min < max * 0.3;
  if (!padded) return [small ? 0 : min, max];
  const pad = Math.max((max - min) * 0.15, max * 0.1, 1);
  return [small ? 0 : Math.max(0, min - pad), max + pad];
}

const EDGE = 2;

/** Value to y in a box `h` tall. A tight flat line sits in the middle instead of on the floor. */
function yScale(values: number[], h: number, padded: boolean) {
  const [lo, hi] = domain(values, padded);
  const flat = !padded && Math.min(...values) === Math.max(...values);
  const range = hi - lo || 1;
  return (v: number) => (flat ? h / 2 : EDGE + (1 - (v - lo) / range) * (h - EDGE * 2));
}

/** The peak sample; the earliest one when several tie. Null when empty. */
export function peakOf(series: [number, number][]): [number, number] | null {
  let best: [number, number] | null = null;
  for (const p of series) if (!best || p[1] > best[1]) best = p;
  return best;
}

/**
 * The viewer graph scaled into `w`×`h`, oldest left. `series` is [unix seconds, viewers].
 * `from` (unix seconds) is the axis start, e.g. the stream start (V106-18). A series that is
 * small next to its peak keeps 0 as the floor; a flat big one doesn't hug the bottom.
 * `padded`: see `domain`.
 */
export function graph(series: [number, number][], w: number, h: number, from?: number, padded = false): Graph {
  if (series.length < 2) return { line: '', area: '', min: 0, max: 0, x0: 0 };
  const x = axis(series, w, from);
  const values = series.map(([, v]) => v);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const y = yScale(values, h, padded);
  const xs = series.map(([t]) => x(t));
  const pts = series.map(([, v], i) => `${round(xs[i])},${round(y(v))}`);
  const line = pts.join(' ');
  const area = `M${round(xs[0])},${h} L${pts.join(' L')} L${round(xs[xs.length - 1])},${h} Z`;
  return { line, area, min, max, x0: round(xs[0]) };
}

/** Index of the sample closest to pixel `x` (same axis as `graph`); -1 when empty. */
export function nearest(series: [number, number][], x: number, w: number, from?: number): number {
  if (!series.length) return -1;
  if (series.length === 1) return 0;
  const at = axis(series, w, from);
  let best = 0;
  let gap = Infinity;
  series.forEach(([t], i) => {
    const d = Math.abs(at(t) - x);
    if (d < gap) {
      gap = d;
      best = i;
    }
  });
  return best;
}

/** x and y of one sample in the same box as `graph`, for a marker. */
export function pointAt(
  series: [number, number][],
  i: number,
  w: number,
  h: number,
  from?: number,
  padded = false,
): [number, number] {
  const x = axis(series, w, from);
  const y = yScale(
    series.map(([, v]) => v),
    h,
    padded,
  );
  return [round(x(series[i][0])), round(y(series[i][1]))];
}

/** Whether pixel `x` is over the "before" band, `before` wide (0 or less: no band). */
export function inBand(x: number, before: number): boolean {
  return before > 0 && x < before;
}
