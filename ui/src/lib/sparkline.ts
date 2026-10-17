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

/** Time to x: `t0` is the left edge, `span` the seconds across `w`. */
function axis(series: [number, number][], w: number, from?: number) {
  const t0 = from ?? series[0][0];
  const span = Math.max(1, series[series.length - 1][0] - t0);
  return (t: number) => ((t - t0) / span) * w;
}

/**
 * The viewer graph scaled into `w`×`h`, oldest left. `series` is [unix seconds, viewers].
 * `from` (unix seconds) is the axis start, e.g. the stream start (V106-18). A series that is
 * small next to its peak keeps 0 as the floor; a flat big one doesn't hug the bottom.
 */
export function graph(series: [number, number][], w: number, h: number, from?: number): Graph {
  if (series.length < 2) return { line: '', area: '', min: 0, max: 0, x0: 0 };
  const x = axis(series, w, from);
  const values = series.map(([, v]) => v);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const floor = min < max * 0.3 ? 0 : min;
  // A flat line sits in the middle instead of on the floor.
  const range = max - floor || 1;
  const pad = 2;
  const xs = series.map(([t]) => x(t));
  const pts = series.map(([, v], i) => {
    const y = max === min ? h / 2 : pad + (1 - (v - floor) / range) * (h - pad * 2);
    return `${round(xs[i])},${round(y)}`;
  });
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
export function place(series: [number, number][], i: number, w: number, h: number, from?: number): [number, number] {
  const x = axis(series, w, from);
  const values = series.map(([, v]) => v);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const floor = min < max * 0.3 ? 0 : min;
  const pad = 2;
  const y = max === min ? h / 2 : pad + (1 - (series[i][1] - floor) / (max - floor || 1)) * (h - pad * 2);
  return [round(x(series[i][0])), round(y)];
}
