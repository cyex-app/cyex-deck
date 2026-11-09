// V108-BD A Twitch badge uploaded while the app is open isn't in the catalog read at join.
// The first line that carries one asks Rust to read that catalog again (Rust throttles it
// to once per 5 minutes per catalog). Each badge asks once per catalog and window, so one
// Twitch never returns doesn't ask on every line.

import { CappedMap } from '../../lib/capped.ts';

const MAX_SEEN = 2000;

export class BadgeMisses {
  private seen = new Set<string>();
  private max: number;

  constructor(max = MAX_SEEN) {
    this.max = max;
  }

  /** True the first time this badge is missing from this catalog (`scope`). */
  first(scope: string, set: string, version: string): boolean {
    const key = `${scope}|${set}/${version}`;
    if (this.seen.has(key)) return false;
    // Past the cap the list starts over: a few badges ask once more.
    if (this.seen.size >= this.max) this.seen.clear();
    this.seen.add(key);
    return true;
  }
}

/** Where a badge is looked up: the source room for Shared Chat refs, else the channel. */
export function missScope(channel: string, room: string | undefined): string {
  return room ? `room:${room}` : channel;
}

/** V110-F84 Keys whose read failed: asked again no sooner than `wait` ms later, so a line
 *  that renders over and over doesn't hammer a failing read. */
export class RetryGate {
  private failed = new CappedMap<string, number>(500);
  private wait: number;

  constructor(wait = 60_000) {
    this.wait = wait;
  }

  /** True when `key` never failed, or failed long enough ago. */
  ok(key: string, now = Date.now()): boolean {
    const at = this.failed.get(key);
    return at === undefined || now - at >= this.wait;
  }

  fail(key: string, now = Date.now()) {
    this.failed.set(key, now);
  }

  done(key: string) {
    this.failed.delete(key);
  }
}
