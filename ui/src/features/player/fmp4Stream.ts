// V108-LL7 Low latency streams every piece of the segment being made (fMP4 too) in 32 KB pieces.
// V108-LL3 had to hold an fMP4 segment from 0,9 s in until it was done: hls.js 1.7.3's
// passthrough remuxer checks every pushed piece against the segment's playlist start
// (`isInvalidInitPts`) and remaps initPTS for a piece a second or more in. That check only runs
// with `accurateTimeOffset` set (passthrough-remuxer.ts: `(accurateTimeOffset || !initPTS) &&
// isInvalidInitPts(...)`), which the stream controller passes per push. So the first piece of a
// segment still carries it (a real clock jump at a segment start is still caught) and the later
// pieces of the same fMP4 segment don't: they keep the segment's initPTS, as TS pieces do.
// The patch wraps the main stream controller's TransmuxerInterface.push (set lazily on the first
// segment), nothing in hls.js itself. hls.js also passes streamed bytes on only once 128 KB have
// gathered (fragment-loader.ts MIN_CHUNK_SIZE): ~0,15 s at 1080p60, ~0,75 s at 480p, a whole segment
// at 160p. 32 KB pieces measured 0,5-0,7 s closer (2026/11/02, ll7-latency.md).

import type HlsType from 'hls.js';

import { ProgressiveLoader } from './progressive.ts';

/** Dev localStorage key: 'base' goes back to the V108-LL6 path (fMP4 held from 0,9 s, 128 KB
 *  pieces); any value also exposes the live hls instance for CDP measurements. */
export const LL7_KEY = 'cyex.ll7';

export function ll7Mode(): string | null {
  try {
    return localStorage.getItem(LL7_KEY);
  } catch {
    return null;
  }
}

/** The V108-LL6 path, for comparison runs. */
export const ll7Old = (): boolean => ll7Mode() === 'base';
/** Piece size: ~40 ms at 6 Mbps, ~0,2 s at 1,4 Mbps (480p), ~1 s at 160p. */
export const LL7_CHUNK = 32 * 1024;

/** The live low latency loader: every finished moof goes on at once, in 32 KB pieces. */
export class StreamLoader extends ProgressiveLoader {
  protected override cut = Infinity;
  protected override chunk: number | null = LL7_CHUNK;
}

interface PushFrag {
  initSegment?: unknown;
  stats: object;
}
interface ChunkMeta {
  id: number;
}
type Push = (...a: unknown[]) => unknown;

/** Whether a push of `frag`'s chunk keeps hls.js's timestamp check: only a segment's first piece
 *  does, or any piece of a segment without an init (TS). */
export function firstPiece(firsts: WeakMap<object, { stats: object; id: number }>, frag: PushFrag, meta: ChunkMeta): boolean {
  if (!frag.initSegment) return true;
  const seen = firsts.get(frag);
  // A retry gets new stats: it starts the segment over.
  if (!seen || seen.stats !== frag.stats || meta.id <= seen.id) {
    firsts.set(frag, { stats: frag.stats, id: meta.id });
    return true;
  }
  return false;
}

/** Wraps `push` (TransmuxerInterface.push's arguments: ..., accurateTimeOffset at 7, chunkMeta at 8). */
export function wrapPush(push: Push): Push {
  const firsts = new WeakMap<object, { stats: object; id: number }>();
  return function (this: unknown, ...a: unknown[]) {
    if (a[7] === true && !firstPiece(firsts, a[4] as PushFrag, a[8] as ChunkMeta)) a[7] = false;
    return push.apply(this, a);
  };
}

/** Installs the wrap on `hls`'s main stream controller; false when hls.js's shape isn't as expected. */
export function streamFmp4(hls: HlsType): boolean {
  const sc = (hls as unknown as { streamController?: Record<string, unknown> }).streamController;
  if (!sc || 'transmuxer' in sc === false) return false;
  let tm: { push?: Push } | null = (sc.transmuxer as { push?: Push } | null) ?? null;
  const wrap = (t: { push?: Push } | null) => {
    if (t && typeof t.push === 'function' && !(t as { __ll7?: true }).__ll7) {
      t.push = wrapPush(t.push.bind(t));
      (t as { __ll7?: true }).__ll7 = true;
    }
    return t;
  };
  wrap(tm);
  Object.defineProperty(sc, 'transmuxer', {
    configurable: true,
    get: () => tm,
    set: (v) => {
      tm = wrap(v as { push?: Push } | null);
    },
  });
  return true;
}

/** Dev only: the live hls instance for CDP measurements (flag set at all). */
export function exposeLive(hls: HlsType): void {
  if (ll7Mode() === null) return;
  (globalThis as unknown as { __ll7hls?: HlsType }).__ll7hls = hls;
}

