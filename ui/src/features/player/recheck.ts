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
