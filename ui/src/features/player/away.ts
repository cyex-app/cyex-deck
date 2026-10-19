// V103-03 A player out of sight, e.g. its tab playing in the background (V102-15), keeps
// its quality for a minute and then drops to the lowest. Back within the minute nothing
// changes, so nothing buffers; back later, it gets its quality again. Out of sight shows
// up as a 0 px player: the tab's slot is `display: none`. One timer per player.
// V107-H1 A hidden page counts too: the window minimized, in the tray or covered.

/** How long a player stays out of sight before it drops to the lowest quality. */
export const AWAY_MS = 60_000;

/** Out of sight: a 0 px player or a hidden page, unless it plays in picture in picture. */
export function outOfSight(zeroPx: boolean, pageHidden: boolean, pip: boolean): boolean {
  return (zeroPx || pageHidden) && !pip;
}

export class Away {
  private timer: ReturnType<typeof setTimeout> | undefined;
  private readonly drop: () => void;
  private readonly ms: number;
  /** Dropped to the lowest quality, until the player shows again. */
  low = false;

  /** `drop` runs once the player has been out of sight for `ms`. */
  constructor(drop: () => void, ms = AWAY_MS) {
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
    }, this.ms);
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
