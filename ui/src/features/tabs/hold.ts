// V102-15 Which tab keeps playing out of sight. Switching away from a tab while its stream
// plays keeps it going, muted or not, as it was, until the tab on screen gets a live stream
// of its own going: an offline tab, a chat-only one, a VOD or a stream that never loads
// leave it playing; both play for a moment while the new one starts. A tab whose live
// stream already plays in its own window stops it at once. Plain functions so they can be
// tested.

export interface Switch {
  /** The tab kept playing now. */
  held: string | null;
  /** The tab shown before, `null` for a view like home or settings. */
  from: string | null;
  /** The tab shown next, `null` for a view. */
  to: string | null;
  /** A stream or video plays in that tab right now (`playingNow` on its videos). */
  playing: (tab: string) => boolean;
  /** That tab's live stream already plays elsewhere: it's in its own window and live. */
  playsElsewhere: (tab: string) => boolean;
}

/** Whether any of a tab's videos plays now. Muted ones count: they keep playing too. */
export function playingNow(videos: Iterable<{ paused: boolean; ended: boolean }>): boolean {
  for (const v of videos) if (!v.paused && !v.ended) return true;
  return false;
}

/** V111-27 Which tabs have a video playing, as media events told it. A tab can leave the page
 * before the switch is looked at (a new tab picked from outside the tab bar: the view is gone
 * when `nextHeld` asks), so what it was doing is remembered, not only read from the page. */
export class PlayingTabs {
  readonly #on = new Set<string>();

  mark(tab: string, on: boolean): void {
    if (on) this.#on.add(tab);
    else this.#on.delete(tab);
  }

  /** Whether `tab` plays. `onPage` is what its view on the page says, or `null` when the view
   * is already gone: then the last thing seen counts, once. */
  resolve(tab: string, onPage: boolean | null): boolean {
    if (onPage !== null) {
      this.mark(tab, onPage);
      return onPage;
    }
    const was = this.#on.has(tab);
    this.#on.delete(tab);
    return was;
  }
}

/** The tab to keep playing after the switch, if any. */
export function nextHeld(s: Switch): string | null {
  if (s.to !== null && (s.to === s.held || s.playsElsewhere(s.to))) return null;
  if (s.from !== null && s.from !== s.to && s.playing(s.from)) return s.from;
  return s.held;
}

/** V111-36 Remembers the tab shown before, so a switch can be decided more than once: the first
 * decision (before the new tab draws) keeps the playing tab, and the repeat once the view has
 * changed (`from === to`) leaves it be. */
export class HoldKeeper {
  #shown: string | null = null;

  /** The tab to keep playing after a switch to `s.to`; `now` is the tab on screen after it. */
  switch(s: Omit<Switch, 'from'>, now: string | null): string | null {
    const held = nextHeld({ ...s, from: this.#shown });
    this.#shown = now;
    return held;
  }
}

export interface Started {
  /** The tab kept playing now. */
  held: string | null;
  /** The tab on screen, `null` for a view or a popped-out tab. */
  shown: string | null;
  /** The tab whose video just started playing, if it's in one. */
  tab: string | null;
  /** That video is a live stream (not a VOD, clip or story). */
  live: boolean;
}

/** The tab kept playing after a video started: the tab on screen getting its live stream
 * going stops it. Muted streams count, as above. */
export function heldAfterStart(s: Started): string | null {
  if (s.held === null || !s.live || s.tab === null) return s.held;
  return s.tab === s.shown && s.tab !== s.held ? null : s.held;
}
