// V108-LL3 The segment still being made plays while it arrives (hls.js progressive), as on
// twitch.tv. MPEG-TS streams it as is. fMP4 (enhanced broadcasting sources, e.g. PSP1G's
// 1080p) can't stream whole: hls.js 1.7.3's passthrough remuxer checks every streamed piece
// against the segment's start and takes one that starts a second or more in (and is shorter
// than that) for a clock jump. It moved the timeline back about a second each segment: the
// new frames overwrote the ones just shown, the buffer shrank under the play head and the
// picture froze ~4 s every ~5 s (2026/10/31). Its demuxer also keeps each piece's last moof
// back for the next one, and the segment's very last moof (2 s in, 0.1 s long) is remuxed alone
// at the end: refused the same way, even for a segment loaded whole. So an fMP4 segment
// streams only its first second, the rest comes in one piece when it's done, and an empty
// moof after it is what the demuxer keeps back (no samples: nothing is appended).

import type { FragmentLoaderContext, HlsConfig, Loader, LoaderCallbacks, LoaderConfiguration, LoaderContext } from 'hls.js';

/** s into a segment a piece may start at and still stream (hls.js refuses 1 s and more). */
export const FMP4_CUT = 0.9;

/** An empty moof: the last box of a segment, held back by hls.js in place of real media. */
export const EMPTY_MOOF = new Uint8Array([0, 0, 0, 8, 0x6d, 0x6f, 0x6f, 0x66]);

interface Box {
  type: string;
  start: number;
  /** Where the box ends; past the data while it's still arriving. */
  end: number;
  /** Where its contents start. */
  body: number;
  /** V108-LL6 Size 0 ("to the end of the file"): its end isn't known while the data streams. */
  open: boolean;
  /** V108-LL6 A size smaller than its own header. */
  broken: boolean;
}

/** u32 at `at` if it lies inside `d` and before `end`, else null (a cut or broken box). */
function u32(d: Uint8Array, at: number, end = d.length): number | null {
  if (at < 0 || at + 4 > Math.min(end, d.length)) return null;
  return ((d[at] << 24) >>> 0) + (d[at + 1] << 16) + (d[at + 2] << 8) + d[at + 3];
}

/** The box at `at`, or null while its header hasn't fully arrived. */
function boxAt(d: Uint8Array, at: number, len = d.length): Box | null {
  if (at + 8 > len) return null;
  let size = u32(d, at, len)!;
  const type = String.fromCharCode(d[at + 4], d[at + 5], d[at + 6], d[at + 7]);
  let body = at + 8;
  let open = false;
  if (size === 1) {
    if (at + 16 > len) return null;
    size = u32(d, at + 8, len)! * 2 ** 32 + u32(d, at + 12, len)!;
    body = at + 16;
  } else if (size === 0) {
    size = len - at;
    open = true;
  }
  const broken = size < body - at;
  if (broken) size = body - at; // step over the header only
  return { type, start: at, end: at + size, body, open, broken };
}

/** The complete boxes in `d` from `from` to `to`. */
function children(d: Uint8Array, from: number, to: number): Box[] {
  const out: Box[] = [];
  for (let at = from; at < to; ) {
    const b = boxAt(d, at, to);
    if (!b || b.end > to) break;
    out.push(b);
    at = b.end;
  }
  return out;
}

const child = (d: Uint8Array, b: Box, type: string) => children(d, b.body, b.end).find((c) => c.type === type);

/** track id → timescale, from an init segment (moov/trak/tkhd + mdia/mdhd). A cut or broken
 *  box gives no entry for its track (never throws): that track's segments come whole. */
export function timescales(init: Uint8Array): Map<number, number> {
  const out = new Map<number, number>();
  const moov = children(init, 0, init.length).find((b) => b.type === 'moov');
  if (!moov) return out;
  for (const trak of children(init, moov.body, moov.end).filter((b) => b.type === 'trak')) {
    const tkhd = child(init, trak, 'tkhd');
    const mdia = child(init, trak, 'mdia');
    const mdhd = mdia && child(init, mdia, 'mdhd');
    if (!tkhd || !mdhd || tkhd.body >= tkhd.end || mdhd.body >= mdhd.end) continue;
    const id = u32(init, tkhd.body + (init[tkhd.body] === 1 ? 20 : 12), tkhd.end);
    const scale = u32(init, mdhd.body + (init[mdhd.body] === 1 ? 20 : 12), mdhd.end);
    if (id !== null && scale) out.set(id, scale);
  }
  return out;
}

/** Seconds a complete moof starts at (its first track with a known timescale), null when unknown. */
export function moofTime(d: Uint8Array, moof: Box, scales: Map<number, number>): number | null {
  for (const traf of children(d, moof.body, moof.end).filter((b) => b.type === 'traf')) {
    const tfhd = child(d, traf, 'tfhd');
    const tfdt = child(d, traf, 'tfdt');
    const id = tfhd ? u32(d, tfhd.body + 4, tfhd.end) : null;
    const scale = id === null ? undefined : scales.get(id);
    if (!tfdt || !scale || tfdt.body >= tfdt.end) continue;
    const t = d[tfdt.body] === 1 ? (u32(d, tfdt.body + 4, tfdt.end) ?? NaN) * 2 ** 32 + (u32(d, tfdt.body + 8, tfdt.end) ?? NaN) : (u32(d, tfdt.body + 4, tfdt.end) ?? NaN);
    return Number.isFinite(t) ? t / scale : null;
  }
  return null;
}

/** One fMP4 segment as it arrives: what may go to hls.js now, and the rest at the end. A piece
 *  goes on once the next moof has arrived and starts before FMP4_CUT, so the held rest always
 *  starts before it too. V108-LL6 Chunks go into one growing buffer (it doubles when full): a
 *  copy of all so far per network chunk was tens of MB of copying per segment on the main
 *  thread. Anything it can't read (a cut, broken or open-ended box) holds the rest, which then
 *  comes whole at the end. */
export class Fmp4Gate {
  private buf = new Uint8Array(1 << 16);
  private len = 0;
  /** Next top-level box to read, in `buf`. */
  private scan = 0;
  private base: number | null = null;
  /** Unsent moofs in `buf` (after the first piece's start). */
  private moofs: number[] = [];
  private holding = false;
  private scales: Map<number, number>;
  /** V108-LL6 Held only because the first moof's track had no timescale yet: nothing sent. */
  private blind = false;

  constructor(scales: Map<number, number>) {
    this.scales = scales;
  }

  /** Bytes for hls.js now, or null. */
  push(chunk: Uint8Array): Uint8Array | null {
    this.append(chunk);
    if (this.holding) return null;
    let cut = 0;
    try {
      cut = this.read();
    } catch {
      this.holding = true;
    }
    return cut > 0 ? this.take(cut) : null;
  }

  /** V108-LL6 The init segment came after the segment began loading (hls.js loads both at
   *  once when the playlist names a new init): a segment held for want of it streams after all.
   *  Before, that segment came whole and the buffer ran dry at its end (TheBurntPeanut,
   *  2026/11/01: a stall after each new init). */
  rescale(scales: Map<number, number>): void {
    if (!scales.size) return;
    // Known before the first moof came: it reads with them from the start.
    this.scales = scales;
    if (!this.blind) return;
    this.blind = false;
    this.holding = false;
    this.scan = 0;
    this.moofs = [];
  }

  /** The rest once the segment is done, with EMPTY_MOOF after it. */
  end(): Uint8Array {
    const out = new Uint8Array(this.len + EMPTY_MOOF.length);
    out.set(this.buf.subarray(0, this.len));
    out.set(EMPTY_MOOF, this.len);
    this.len = 0;
    this.scan = 0;
    this.moofs = [];
    return out;
  }

  /** Reads the boxes that came; where the bytes for hls.js end (0: none yet). */
  private read(): number {
    const d = this.buf.subarray(0, this.len);
    let cut = 0;
    for (;;) {
      const b = boxAt(d, this.scan);
      if (!b) break;
      // Open-ended or broken: where the next box starts is unknown.
      if (b.open || b.broken) {
        this.holding = true;
        break;
      }
      if (b.type === 'moof') {
        if (b.end > d.length) break; // read it once it's whole
        const t = moofTime(d, b, this.scales);
        if (t === null) {
          this.blind = this.base === null;
          this.holding = true;
          break;
        }
        this.base ??= t;
        if (t - this.base >= FMP4_CUT) {
          this.holding = true;
          break;
        }
        // Everything before this moof is whole fragments that start before the cut.
        if (this.moofs.length) cut = b.start;
        this.moofs.push(b.start);
      }
      this.scan = b.end;
    }
    return cut;
  }

  private append(chunk: Uint8Array): void {
    const need = this.len + chunk.length;
    if (need > this.buf.length) {
      let cap = this.buf.length * 2;
      while (cap < need) cap *= 2;
      const next = new Uint8Array(cap);
      next.set(this.buf.subarray(0, this.len));
      this.buf = next;
    }
    this.buf.set(chunk, this.len);
    this.len = need;
  }

  /** The first `n` bytes, in their own buffer (hls.js may hand it to its worker). */
  private take(n: number): Uint8Array {
    const out = this.buf.slice(0, n);
    this.buf.copyWithin(0, n, this.len);
    this.len -= n;
    this.scan -= n;
    this.moofs = this.moofs.filter((m) => m >= n).map((m) => m - n);
    return out;
  }
}

type FragLoader = Loader<FragmentLoaderContext>;

/** The fragment loader for progressive live: hls.js's own (fetch, set by `progressive`) with
 *  fMP4 segments gated (Fmp4Gate). TS and everything else pass straight through. */
export class ProgressiveLoader implements FragLoader {
  private readonly inner: FragLoader;

  constructor(config: HlsConfig) {
    this.inner = new config.loader(config) as unknown as FragLoader;
  }

  get context(): FragmentLoaderContext | null {
    return this.inner.context;
  }

  get stats() {
    return this.inner.stats;
  }

  destroy(): void {
    this.inner.destroy();
  }

  abort(): void {
    this.inner.abort();
  }

  getCacheAge(): number | null {
    return this.inner.getCacheAge?.() ?? null;
  }

  getResponseHeader(name: string): string | null {
    return this.inner.getResponseHeader?.(name) ?? null;
  }

  load(context: FragmentLoaderContext, config: LoaderConfiguration, callbacks: LoaderCallbacks<FragmentLoaderContext>): void {
    const onProgress = callbacks.onProgress;
    const init = context.frag.initSegment;
    if (!onProgress || !init || context.frag.sn === 'initSegment') return this.inner.load(context, config, callbacks);
    // V108-LL6 A new init loads alongside its first segment: read once it's there (until then,
    // or if it never reads, the gate holds the segment and it comes whole).
    let scales = init.data ? timescales(init.data) : new Map<number, number>();
    let gate = new Fmp4Gate(scales);
    let retry = 0;
    const send = (stats: Parameters<typeof onProgress>[0], ctx: LoaderContext, data: Uint8Array | null, nd: Parameters<typeof onProgress>[3]) => {
      if (data) onProgress(stats, ctx as FragmentLoaderContext, data.buffer as ArrayBuffer, nd);
    };
    this.inner.load(context, config, {
      ...callbacks,
      onProgress: (stats, ctx, data, nd) => {
        // A retry starts the segment over.
        if (stats.retry !== retry) {
          retry = stats.retry;
          gate = new Fmp4Gate(scales);
        }
        if (!scales.size && init.data) {
          scales = timescales(init.data);
          gate.rescale(scales);
        }
        send(stats, ctx, gate.push(new Uint8Array(data as ArrayBuffer)), nd);
      },
      onSuccess: (res, stats, ctx, nd) => {
        send(stats, ctx, gate.end(), nd);
        callbacks.onSuccess(res, stats, ctx, nd);
      },
    });
  }
}
