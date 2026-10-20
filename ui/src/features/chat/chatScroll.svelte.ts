// V107-R15 Chat scroll state (1.8, C5, V101-15/31, V103-11/12, V107-M6), out of MessageList.
// The list tells it what happened (scrolls, wheel, hover, keys, menus); it says whether chat
// follows, is paused, how many lines came since, which lines are drawn, and what the list
// should do (go to the bottom, draw older lines). The DOM, the slide Animation and the
// ResizeObserver stay in MessageList.
//
// Rules:
// - Only the user leaves the bottom (wheel up, keys, the scrollbar, a jump). Anything else
//   that moves the list while following snaps back down.
// - Following starts again on the way down at the bottom, or when nothing is left to scroll.
// - Paused: hover (unless the mouse rested, V101-15), the pause key, or a menu from this list.
// - Scrolled up or paused, the drawn window holds; following again for SHRINK_MS shrinks it.
// - The first placement at the bottom ignores the pause (v1.07 t1): a chat that opens under a
//   resting mouse still starts at its newest line.

import { glideMs } from './glide.ts';
import { around, hold, older, SHRINK_MS, startOf, type From } from './lineWindow.ts';

type Item = { id: string; ts: number };

/** The list's scroll position, in px. */
export interface Metrics {
  top: number;
  height: number;
  client: number;
}

export interface ScrollOpts {
  pauseOnHover: boolean;
  /** Tenths of a second the mouse rests before chat moves on; 0: never. */
  pauseStillTenths: number;
}

export interface Clock {
  now(): number;
  setTimeout(fn: () => void, ms: number): unknown;
  clearTimeout(t: unknown): void;
}

const realClock: Clock = {
  now: () => performance.now(),
  setTimeout: (fn, ms) => setTimeout(fn, ms),
  clearTimeout: (t) => clearTimeout(t as ReturnType<typeof setTimeout>),
};

export interface ScrollSource<T extends Item> {
  /** What the list shows (filters applied). */
  lines: () => readonly T[];
  /** Every message, to count what came in. */
  messages: () => readonly { id: string }[];
  /** The window shrank: the list goes back to the bottom if it still follows. */
  onshrink: () => void;
}

/** What the list should do after a scroll. */
export type Intent = 'bottom' | 'older' | null;

/** Pixels from the bottom that still count as at the bottom. */
const NEAR = 24;

export class ChatScroll<T extends Item = Item> {
  #following = $state(true);
  #below = $state(false);
  #hovering = $state(false);
  #still = $state(false);
  #keyHeld = $state(false);
  #menu = $state(false);
  #opts = $state.raw<ScrollOpts>({ pauseOnHover: false, pauseStillTenths: 0 });
  /** Last message when the pause began. */
  #pausedAt = $state<string | undefined>();
  /** Last message when following ended. */
  #leftAt = $state<string | undefined>();
  #from = $state.raw<From>(null);

  #src: ScrollSource<T>;
  #clock: Clock;
  #stillTimer: unknown;
  #shrinkTimer: unknown;
  #placed = false;
  #growing = false;
  #dragging = false;
  #lastTop = 0;
  #lastLines = -Infinity;
  #slideEnds = 0;
  /** What `#settle` last saw, to act on changes only. */
  #seen = { following: true, paused: false, from: null as From };

  constructor(src: ScrollSource<T>, opts: ScrollOpts, clock: Clock = realClock) {
    this.#src = src;
    this.#opts = opts;
    this.#clock = clock;
  }

  /** New lines are kept in view. */
  get following(): boolean {
    return this.#following;
  }

  get paused(): boolean {
    return (this.#hovering && this.#opts.pauseOnHover && !this.#still) || this.#keyHeld || this.#menu;
  }

  /** Follow new lines now: following, and paused only once placed at the bottom. */
  get sticks(): boolean {
    return this.#following && (!this.paused || !this.#placed);
  }

  /** Lines that came in since chat stopped following (scrolled up or paused). */
  get fresh(): number {
    const id = !this.#following ? this.#leftAt : this.#pausedAt;
    if (!id) return 0;
    const ms = this.#src.messages();
    for (let i = ms.length - 1; i >= 0; i--) if (ms[i].id === id) return ms.length - 1 - i;
    return ms.length;
  }

  /** V101-31 The "new / Paused / More messages" pill shows. */
  get pill(): boolean {
    if (!this.#below) return false;
    if (!this.#following) return true;
    return this.paused && this.#pausedAt !== undefined && this.#src.messages().at(-1)?.id !== this.#pausedAt;
  }

  /** V107-M6 The lines drawn. */
  get drawn(): readonly T[] {
    const shown = this.#src.lines();
    const start = startOf(shown, this.#from);
    return start > 0 ? shown.slice(start) : shown;
  }

  configure(opts: ScrollOpts): void {
    this.#opts = opts;
    this.#settle();
  }

  /** The mouse came over the list. */
  enter(): void {
    this.#hovering = true;
    this.move();
  }

  /** V101-15 The mouse moved: chat pauses again, until it rests. */
  move(): void {
    this.#still = false;
    this.#clock.clearTimeout(this.#stillTimer);
    const { pauseOnHover, pauseStillTenths } = this.#opts;
    if (pauseOnHover && pauseStillTenths)
      this.#stillTimer = this.#clock.setTimeout(() => {
        this.#still = true;
        this.#settle();
      }, pauseStillTenths * 100);
    this.#settle();
  }

  exit(): void {
    this.#hovering = false;
    this.#still = false;
    this.#clock.clearTimeout(this.#stillTimer);
    this.#settle();
  }

  /** C5 The pause key is down or up. */
  key(held: boolean): void {
    this.#keyHeld = held;
    this.#settle();
  }

  /** A menu opened from this list is open. */
  menu(open: boolean): void {
    this.#menu = open;
    this.#settle();
  }

  /** The user drags the scrollbar. */
  drag(on: boolean): void {
    this.#dragging = on;
  }

  /** Wheel up, keys up, while there's something to scroll. */
  leave(): void {
    this.#follow(false);
  }

  /** The list was measured (after it moved itself). */
  measured(m: Metrics): void {
    this.#below = m.top + m.client < m.height - NEAR;
  }

  /** A scroll event. Says whether to go back to the bottom or draw older lines. */
  scrolled(m: Metrics): Intent {
    this.measured(m);
    const down = m.top >= this.#lastTop;
    this.#lastTop = m.top;
    let out: Intent = null;
    if (!this.#below && (down || m.height <= m.client)) this.#follow(true);
    else if (this.#dragging) this.#follow(false);
    else if (this.#following && !this.paused) out = 'bottom';
    if (!this.#following && m.top < m.client && this.#older()) out = 'older';
    return out;
  }

  /** The older lines from a scroll are on screen. */
  grew(): void {
    this.#growing = false;
  }

  /** The list reached the bottom with something to scroll (v1.07 t1). */
  landed(): void {
    this.#placed = true;
  }

  /** The pill: back to the newest line, unpaused. */
  toNewest(): void {
    this.#hovering = false;
    this.#keyHeld = false;
    this.#follow(true);
  }

  /** V107-M6 A jump to a line that isn't drawn: draws it. False when there's nothing to draw. */
  drawAround(id: string): boolean {
    const shown = this.#src.lines();
    const next = around(shown, this.#from, shown.findIndex((m) => m.id === id));
    if (next === undefined) return false;
    this.#from = next;
    this.#settle();
    return true;
  }

  /** The list jumped to a line: following would snap straight back down. */
  jumped(): void {
    this.#follow(false);
  }

  /**
   * V103-12 How long new lines slide, in ms (glide.ts); 0 just shows them. `carry`: the lines
   * grew while a slide played (`playing`), which keeps its end time.
   */
  glide(distance: number, viewport: number, carry: boolean, playing: boolean): number {
    if (!(distance >= 1)) return 0;
    const now = this.#clock.now();
    const ms = carry
      ? Math.min(playing ? this.#slideEnds - now : 0, glideMs(distance, viewport, Infinity))
      : glideMs(distance, viewport, now - this.#lastLines);
    if (!carry) this.#lastLines = now;
    if (ms > 0) this.#slideEnds = now + ms;
    return ms;
  }

  dispose(): void {
    this.#clock.clearTimeout(this.#stillTimer);
    this.#clock.clearTimeout(this.#shrinkTimer);
  }

  #older(): boolean {
    if (this.#growing) return false;
    const next = older(this.#src.lines(), this.#from);
    if (next === undefined) return false;
    this.#growing = true;
    this.#from = next;
    this.#settle();
    return true;
  }

  #follow(on: boolean): void {
    this.#following = on;
    if (on) this.#leftAt = undefined;
    else this.#leftAt ??= this.#src.messages().at(-1)?.id ?? '';
    this.#settle();
  }

  /** Keeps what hangs on following, paused and the window in step. */
  #settle(): void {
    const following = this.#following;
    const paused = this.paused;
    const last = this.#seen;
    if (!paused) this.#pausedAt = undefined;
    else this.#pausedAt ??= this.#src.messages().at(-1)?.id ?? '';
    // Scrolled up or paused: what's drawn stays put.
    if ((following !== last.following || (following && paused !== last.paused)) && (!following || paused))
      this.#from = hold(this.#src.lines(), this.#from);
    const from = this.#from;
    // Back at the bottom for a moment: only the newest lines stay drawn.
    if (following !== last.following || paused !== last.paused || from !== last.from) {
      this.#clock.clearTimeout(this.#shrinkTimer);
      this.#shrinkTimer = undefined;
      if (following && !paused && from)
        this.#shrinkTimer = this.#clock.setTimeout(() => {
          this.#shrinkTimer = undefined;
          this.#from = null;
          this.#settle();
          this.#src.onshrink();
        }, SHRINK_MS);
    }
    this.#seen = { following, paused, from };
  }
}
