import type { HlsConfig } from 'hls.js';

// V107-Md After the quick retries gave up, a network failure is rechecked quietly while Twitch
// still lists the stream: every RECHECK_MS, until it plays, goes offline or the page hides.

export const RECHECK_MS = 15_000;
/** The fresh-session tries (3 s, 6 s, 9 s) before the quiet recheck takes over. */
export const QUICK_TRIES = 3;

export interface RecheckInput {
  readonly failure: string | null;
  /** Quick retries used so far. */
  readonly tries: number;
  /** Twitch still lists the stream as live. */
  readonly listed: boolean;
  /** The page is hidden. */
  readonly hidden: boolean;
}

/** Wait before the next quiet recheck, or null when none is due. */
export function recheckDelay(i: RecheckInput): number | null {
  return i.failure === 'network' && i.tries >= QUICK_TRIES && i.listed && !i.hidden ? RECHECK_MS : null;
}

/** V110-F83 Wait before the next offline recheck: 15, 30, then 60 s while it keeps failing;
 *  null while the page is hidden (it looks again when shown). */
export function offlineDelay(misses: number, hidden: boolean): number | null {
  return hidden ? null : Math.min(RECHECK_MS * 2 ** Math.max(0, misses), 4 * RECHECK_MS);
}

/** V110-F96 hls.js asked a failing master playlist twice per start (one more try after an error
 *  status, 1 s later) on top of our own rechecks, about 14 asks a minute for a stream whose
 *  playlist fails. A failed ask now ends the start at once; the quick tries (3, 6, 9 s) and the
 *  15 s recheck above do the retrying, and recover within 15 s of the server healing. A hang
 *  still gets one more try. */
export const MANIFEST_POLICY: HlsConfig['manifestLoadPolicy'] = {
  default: {
    maxTimeToFirstByteMs: Infinity,
    maxLoadTimeMs: 20_000,
    timeoutRetry: { maxNumRetry: 1, retryDelayMs: 0, maxRetryDelayMs: 0 },
    errorRetry: { maxNumRetry: 0, retryDelayMs: 0, maxRetryDelayMs: 0 },
  },
};
