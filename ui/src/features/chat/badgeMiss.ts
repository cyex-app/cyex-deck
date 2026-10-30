// V108-BD A Twitch badge uploaded while the app is open isn't in the catalog read at join.
// The first line that carries one asks Rust to read that catalog again (Rust throttles it
// to once per 5 minutes per catalog). Each badge asks once per catalog and window, so one
// Twitch never returns doesn't ask on every line.

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
