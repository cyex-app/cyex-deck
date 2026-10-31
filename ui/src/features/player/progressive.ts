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
}

/** The box at `at`, or null while its header hasn't fully arrived. */
function boxAt(d: Uint8Array, at: number): Box | null {
  if (at + 8 > d.length) return null;
  const v = new DataView(d.buffer, d.byteOffset, d.byteLength);
  let size = v.getUint32(at);
  const type = String.fromCharCode(d[at + 4], d[at + 5], d[at + 6], d[at + 7]);
  let body = at + 8;
  if (size === 1) {
    if (at + 16 > d.length) return null;
    size = Number(v.getBigUint64(at + 8));
    body = at + 16;
  } else if (size === 0) {
    size = d.length - at;
  }
  if (size < body - at) size = body - at; // broken: step over the header only
  return { type, start: at, end: at + size, body };
}

/** The complete boxes in `d` from `from` to `to`. */
function children(d: Uint8Array, from: number, to: number): Box[] {
  const out: Box[] = [];
  for (let at = from; at < to; ) {
    const b = boxAt(d, at);
    if (!b || b.end > to) break;
    out.push(b);
    at = b.end;
  }
  return out;
}

const child = (d: Uint8Array, b: Box, type: string) => children(d, b.body, b.end).find((c) => c.type === type);
const u32 = (d: Uint8Array, at: number) => new DataView(d.buffer, d.byteOffset, d.byteLength).getUint32(at);

/** track id → timescale, from an init segment (moov/trak/tkhd + mdia/mdhd). */
export function timescales(init: Uint8Array): Map<number, number> {
  const out = new Map<number, number>();
  const moov = children(init, 0, init.length).find((b) => b.type === 'moov');
  if (!moov) return out;
  for (const trak of children(init, moov.body, moov.end).filter((b) => b.type === 'trak')) {
    const tkhd = child(init, trak, 'tkhd');
    const mdia = child(init, trak, 'mdia');
    const mdhd = mdia && child(init, mdia, 'mdhd');
    if (!tkhd || !mdhd) continue;
    const id = u32(init, tkhd.body + (init[tkhd.body] === 1 ? 20 : 12));
    const scale = u32(init, mdhd.body + (init[mdhd.body] === 1 ? 20 : 12));
    if (scale > 0) out.set(id, scale);
  }
  return out;
}

/** Seconds a complete moof starts at (its first track with a known timescale), null when unknown. */
export function moofTime(d: Uint8Array, moof: Box, scales: Map<number, number>): number | null {
  for (const traf of children(d, moof.body, moof.end).filter((b) => b.type === 'traf')) {
    const tfhd = child(d, traf, 'tfhd');
    const tfdt = child(d, traf, 'tfdt');
    const scale = tfhd && scales.get(u32(d, tfhd.body + 4));
    if (!tfdt || !scale) continue;
    const v = new DataView(d.buffer, d.byteOffset, d.byteLength);
    const t = d[tfdt.body] === 1 ? Number(v.getBigUint64(tfdt.body + 4)) : v.getUint32(tfdt.body + 4);
    return t / scale;
  }
  return null;
}

/** One fMP4 segment as it arrives: what may go to hls.js now, and the rest at the end. A piece
 *  goes on once the next moof has arrived and starts before FMP4_CUT, so the held rest always
 *  starts before it too. */
export class Fmp4Gate {
  private buf = new Uint8Array(0);
  /** Next top-level box to read, in `buf`. */
  private scan = 0;
  private base: number | null = null;
  /** Unsent moofs in `buf` (after the first piece's start). */
  private moofs: number[] = [];
  private holding = false;
  private readonly scales: Map<number, number>;

  constructor(scales: Map<number, number>) {
    this.scales = scales;
  }

  /** Bytes for hls.js now, or null. */
  push(chunk: Uint8Array): Uint8Array | null {
    const next = new Uint8Array(this.buf.length + chunk.length);
    next.set(this.buf);
    next.set(chunk, this.buf.length);
    this.buf = next;
    if (this.holding) return null;
    let cut = 0;
    for (;;) {
      const b = boxAt(this.buf, this.scan);
      if (!b) break;
      if (b.type === 'moof') {
        if (b.end > this.buf.length) break; // read it once it's whole
        const t = moofTime(this.buf, b, this.scales);
        if (t === null) {
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
    return cut > 0 ? this.take(cut) : null;
  }

  /** The rest once the segment is done, with EMPTY_MOOF after it. */
  end(): Uint8Array {
    const rest = this.take(this.buf.length);
    const out = new Uint8Array(rest.length + EMPTY_MOOF.length);
    out.set(rest);
    out.set(EMPTY_MOOF, rest.length);
    return out;
  }

  private take(n: number): Uint8Array {
    const out = this.buf.slice(0, n);
    this.buf = this.buf.slice(n);
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
    // No timescales (init not read yet): the gate holds it all, it comes whole at the end.
    const scales = init.data ? timescales(init.data) : new Map<number, number>();
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
        send(stats, ctx, gate.push(new Uint8Array(data as ArrayBuffer)), nd);
      },
      onSuccess: (res, stats, ctx, nd) => {
        send(stats, ctx, gate.end(), nd);
        callbacks.onSuccess(res, stats, ctx, nd);
      },
    });
  }
}
