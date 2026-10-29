// V103-03 A player out of sight, e.g. its tab playing in the background (V102-15), keeps
// its quality for a minute and then drops to the lowest. Back within the minute nothing
// changes, so nothing buffers; back later, it gets its quality again. Out of sight shows
// up as a 0 px player: the tab's slot is `display: none`. One timer per player.
// V107-H1 A hidden page counts too: the window minimized, in the tray or covered.

/** How long a player stays out of sight before it drops to the lowest quality. */
export const AWAY_MS = 60_000;

// V108-N22 (with P16) Save resources: what a player out of sight does, and after how long.
/** Nothing, the lowest video quality, or audio only. */
export type AwayMode = 'off' | 'lowest' | 'audio';

/** Delays the setting offers, in seconds. */
export const AWAY_SECS = [15, 30, 60, 120, 300, 600] as const;

/** The setting's seconds as a timer length; anything off the list reads as a minute. */
export function awayMs(secs: number): number {
  return Number.isFinite(secs) && secs >= 5 && secs <= 3600 ? Math.round(secs) * 1000 : AWAY_MS;
}

/** The option a stored delay shows as: the nearest one on the list. */
export function awayChoice(secs: number): number {
  return AWAY_SECS.reduce((a, b) => (Math.abs(b - secs) < Math.abs(a - secs) ? b : a));
}

/** Out of sight: a 0 px player or a hidden page, unless it plays in picture in picture. */
export function outOfSight(zeroPx: boolean, pageHidden: boolean, pip: boolean): boolean {
  return (zeroPx || pageHidden) && !pip;
}

export class Away {
  private timer: ReturnType<typeof setTimeout> | undefined;
  private readonly drop: () => void;
  private readonly ms: number | (() => number);
  /** Dropped to the lowest quality, until the player shows again. */
  low = false;

  /** `drop` runs once the player has been out of sight for `ms`. */
  constructor(drop: () => void, ms: number | (() => number) = AWAY_MS) {
    this.drop = drop;
    this.ms = ms;
  }

  /** The player left the screen. A timer already running keeps its start. */
  hide(): void {
    if (this.timer !== undefined || this.low) return;
    this.timer = setTimeout(() => {
      this.timer = undefined;
      this.low = true;
      this.drop();
    }, typeof this.ms === 'function' ? this.ms() : this.ms);
  }

  /** Back on screen. True when it had dropped and needs its quality back. */
  show(): boolean {
    this.stop();
    const dropped = this.low;
    this.low = false;
    return dropped;
  }

  /** Cancels the timer, e.g. when the player closes. */
  stop(): void {
    clearTimeout(this.timer);
    this.timer = undefined;
  }
}
