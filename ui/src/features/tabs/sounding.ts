// V106-25 The videos playing in this window, by tile, and whether each would be heard
// (its own player not muted, nothing over it). The tab mute isn't part of it.

export interface MediaLike extends EventTarget {
  readonly paused: boolean;
  readonly ended: boolean;
}

interface Entry {
  tileId: string;
  playing: boolean;
  wouldPlay: () => boolean;
}

const EVENTS = ['play', 'playing', 'pause', 'ended', 'emptied'] as const;

export class Sounding {
  #next = 0;
  readonly #entries: Map<number, Entry>;

  /** A reactive map (SvelteMap) makes `wouldPlay` reactive. */
  constructor(entries: Map<number, Entry> = new Map()) {
    this.#entries = entries;
  }

  /** Watches one video until the returned stop runs. `wouldPlay` is read when asked, so it may be reactive. */
  watch(tileId: string, media: MediaLike, wouldPlay: () => boolean): () => void {
    const key = this.#next++;
    let last: boolean | undefined;
    const sync = () => {
      const playing = !media.paused && !media.ended;
      // 'play' and 'playing' both fire: only a change reaches the speakers.
      if (playing === last) return;
      last = playing;
      this.#entries.set(key, { tileId, playing, wouldPlay });
    };
    for (const e of EVENTS) media.addEventListener(e, sync);
    sync();
    return () => {
      for (const e of EVENTS) media.removeEventListener(e, sync);
      this.#entries.delete(key);
    };
  }

  /** Something in these tiles would be heard now. */
  wouldPlay(tileIds: ReadonlySet<string>): boolean {
    for (const e of this.#entries.values()) if (e.playing && tileIds.has(e.tileId) && e.wouldPlay()) return true;
    return false;
  }
}
