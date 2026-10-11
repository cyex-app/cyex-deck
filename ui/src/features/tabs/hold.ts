// V102-15 Which tab keeps playing out of sight. Switching away from a tab while its stream
// plays keeps it going, unmuted as it was, as long as what shows next has no live stream of
// its own: a chat-only tab, an offline one, home or settings. A tab with a live stream on
// screen stops it. Plain function so it can be tested.

export interface Switch {
  /** The tab kept playing now. */
  held: string | null;
  /** The tab shown before, `null` for a view like home or settings. */
  from: string | null;
  /** The tab shown next, `null` for a view. */
  to: string | null;
  /** A stream or video plays in that tab right now, and isn't muted. */
  playing: (tab: string) => boolean;
  /** That tab shows a stream and the channel is live. */
  liveViewer: (tab: string) => boolean;
}

/** The tab to keep playing after the switch, if any. */
export function nextHeld(s: Switch): string | null {
  if (s.to !== null && (s.to === s.held || s.liveViewer(s.to))) return null;
  if (s.from !== null && s.from !== s.to && s.playing(s.from)) return s.from;
  return s.held;
}
