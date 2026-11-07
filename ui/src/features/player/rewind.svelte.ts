// V1061-03 Twitch's Stream Rewind for one live player: whether Twitch offers it, the
// stream's clock, and the moves between the live stream and the broadcast's VOD.
// V108-N24 Without it, the replay of what played here: same bar, keys and chat bar, only
// from where the replay starts. Two sources, one clock: seconds since the stream started.

import { playerIpc } from '../../ipc/player';
import { onShown, pageHidden } from '../../lib/awake';
import type { Engine } from './engine.svelte';
import { COALESCE_MS, DVR_GAP, DVR_RETRY_MS, DVR_YOUNG, dvrTarget, edgeAt, sameStart, stepTarget, uptimeAt } from './liveEdge';
import { replayBarStart } from './replayClock';
import type { SeekResult } from './liveKeys';
import { clearRewound, hasOwn, setRewound } from './rewound.svelte';

export interface RewindHost {
  engine(): Engine | undefined;
  /** LivePlayer's `start`: live when `at` is null, else the VOD (or the replay) from `at` s. */
  open(at: number | null): Promise<void>;
  /** V108-N24 The live session playing now, whose replay this is. */
  liveId(): string | null;
}

/** Where going back plays from. */
export type RewindMode = 'dvr' | 'replay';

/** V108-N24 How often the replay's range is read while shown. */
const RANGE_EVERY_S = 2;

export class Rewind {
  vodId = $state<string | null>(null);
  uptime = $state(0);
  ended = $state(false);

  private enabled = $state(false);
  /** V108-N24 The replay setting, minutes (0 off). */
  replayMinutes = $state(0);
  /** V108-N24 What the replay holds, ms; null while it holds nothing. */
  private range = $state.raw<{ from: number; to: number; keep: number } | null>(null);
  private ranges = 0;
  private ticks = 0;
  private startedAt: string | null = null;
  private account: string | null = null;
  private key = '';
  /** Only the newest answer from Twitch counts. */
  private asks = 0;
  private retry: ReturnType<typeof setTimeout> | undefined;
  /** Key presses within COALESCE_MS add up and move once. */
  private pending: number | 'live' | null = null;
  private flushTimer: ReturnType<typeof setTimeout> | undefined;
  private stalls = 0;
  private endTimer: ReturnType<typeof setTimeout> | undefined;
  /** Q21 a The VOD ran out after the stream ended: the stream side may let go. */
  private done = false;
  private hushPaused = false;
  private readonly clock: ReturnType<typeof setInterval>;
  private readonly unShown: () => void;

  constructor(
    private readonly host: RewindHost,
    private readonly login: string,
  ) {
    this.clock = setInterval(() => this.tick(), 1000);
    // V107-H1 Shown again: the clock and the chat bar catch up at once.
    this.unShown = onShown((hidden) => hidden || this.tick());
  }

  /** From LivePlayer's `$effect`: asks Twitch again when one of these changes. */
  watch(o: { account: string | null; startedAt: string | null; enabled: boolean; replay: number }): void {
    this.replayMinutes = o.replay;
    if (o.replay <= 0) this.range = null;
    const key = `${o.account}|${o.startedAt}|${o.enabled}`;
    if (key === this.key) return;
    this.key = key;
    const before = this.startedAt;
    this.account = o.account;
    this.enabled = o.enabled;
    if (!o.enabled) {
      if (o.startedAt) this.startedAt = o.startedAt;
      this.forget();
      if (this.inDvr) void this.toLive();
      return;
    }
    if (o.startedAt === null) {
      // Q21 a Over: the VOD plays to its end, the clock stops.
      if (this.inVod) this.ended = true;
      return;
    }
    this.startedAt = o.startedAt;
    this.ended = false;
    this.done = false;
    clearTimeout(this.endTimer);
    this.endTimer = undefined;
    this.uptime = uptimeAt(o.startedAt, Date.now());
    // V109-F72 By the instant: the GQL and the Helix fallback write the same start differently
    // (`Z` or an offset, fractions, as V108-F56h found), which isn't a restart.
    if (before !== null && !sameStart(before, o.startedAt) && this.inVod) {
      // A-L6 A restart is a new stream with a new VOD.
      this.forget();
      const e = this.host.engine();
      if (e) e.note = 'The stream restarted. Back to live.';
      void this.toLive();
    }
    void this.ask();
  }

  /** Twitch's rewind when it's granted (it has the whole broadcast), else the replay once it
   *  holds more than the live gap. */
  get mode(): RewindMode | null {
    if (this.enabled && this.vodId !== null) return 'dvr';
    if (this.replayMinutes > 0 && this.range && this.secs(this.range.from) < this.uptime - DVR_GAP - 2) return 'replay';
    return null;
  }

  get on(): boolean {
    return this.mode !== null;
  }

  /** Rewound: the rewind VOD or the replay plays. */
  get inVod(): boolean {
    return this.host.engine()?.dvr ?? false;
  }

  /** The rewind VOD plays (not the replay). */
  private get inDvr(): boolean {
    const e = this.host.engine();
    return !!e?.dvr && !e.replay;
  }

  /** V108-N24 The earliest second that can play: the replay's bar start, 0 for Twitch's rewind. */
  get min(): number {
    return this.start;
  }

  /** V108-N24 Where the bar starts: the whole broadcast, or what the replay holds (user,
   *  2026/10/31: a dimmed part for what isn't recorded yet crept across the bar every second).
   *  V109-F72 Full, it slides with the clock instead of jumping with each trimmed segment. */
  get start(): number {
    if (this.mode !== 'replay' || !this.range) return 0;
    return Math.max(0, replayBarStart(this.secs(this.range.from), this.edge, this.range.keep / 1000));
  }

  /** V109-F72 The live point for the bar, to the ms when it's read (`uptime` is whole seconds,
   *  once a second, and waits while hidden). Read with the play head, it moves with it. */
  get edge(): number {
    const up = this.uptime;
    // Read again on each of the engine's time updates, not only on the 1 s tick.
    void this.host.engine()?.time;
    if (this.ended || !this.startedAt) return up;
    return Math.max(up, edgeAt(this.startedAt, Date.now()));
  }

  /** V108-N24 The stream's start in ms: the replay's clock. */
  get origin(): number {
    const t = this.startedAt ? Date.parse(this.startedAt) : NaN;
    return Number.isNaN(t) ? 0 : t;
  }

  /** Where the player is, in seconds since the stream started. */
  get pos(): number {
    const e = this.host.engine();
    return this.inVod && e ? e.time : this.edge;
  }

  get behind(): number | null {
    const e = this.host.engine();
    if (this.inVod && e) return Math.max(0, this.uptime - e.time);
    return e?.behind ?? null;
  }

  /** Q21 a Over: Twitch says so, or the player found it offline (the VOD ran out while Twitch may still say live). */
  get over(): boolean {
    return this.ended || this.host.engine()?.failure === 'offline';
  }

  get atLive(): boolean {
    return !this.inVod && (this.host.engine()?.atLive ?? true);
  }

  /** The bar. */
  seekTo(t: number): void {
    if (!this.on) return;
    this.pending = null;
    clearTimeout(this.flushTimer);
    this.go(dvrTarget(t, this.uptime, this.min));
  }

  /** Keys. null: no rewind here, the key stays with the page (Q29). */
  seekBy(s: number): SeekResult | null {
    if (!this.on) return null;
    const base = typeof this.pending === 'number' ? this.pending : this.pending === 'live' ? this.uptime : this.pos;
    const { at, edge } = stepTarget(base, s, this.uptime, this.min);
    // Live is off after the end: nothing moves.
    if (at === 'live' && this.over) return { behind: Math.max(0, this.uptime - base), edge: null };
    this.pending = at;
    clearTimeout(this.flushTimer);
    this.flushTimer = setTimeout(() => this.flush(), COALESCE_MS);
    return { behind: at === 'live' ? 'live' : this.uptime - at, edge };
  }

  async toLive(): Promise<void> {
    if (this.ended) return;
    this.pending = null;
    clearTimeout(this.flushTimer);
    const e = this.host.engine();
    // rd-2 The engine reopens live from the rewind VOD, playing even when paused there.
    if (this.inVod && !e) return this.host.open(null);
    await e?.toLive(true);
  }

  /** Q22 b A clip over the rewound stream pauses it; closing it plays on from there. At
   *  live the stream keeps playing muted, as before (1.20). */
  hush(on: boolean): void {
    const e = this.host.engine();
    if (!e) return;
    if (on && this.inVod && !e.paused) {
      this.hushPaused = true;
      e.pause();
    } else if (!on && this.hushPaused) {
      this.hushPaused = false;
      void e.resume();
    }
  }

  destroy(): void {
    clearInterval(this.clock);
    this.unShown();
    clearTimeout(this.retry);
    clearTimeout(this.flushTimer);
    clearTimeout(this.endTimer);
    this.asks += 1;
    clearRewound(this);
  }

  /** No rewind until Twitch says so again. */
  private forget(): void {
    clearTimeout(this.retry);
    this.asks += 1;
    this.vodId = null;
  }

  private async ask(): Promise<void> {
    clearTimeout(this.retry);
    const n = ++this.asks;
    let id: string | null = null;
    try {
      id = await playerIpc.rewind(this.login, this.account);
    } catch {
      id = null;
    }
    if (n !== this.asks) return;
    this.vodId = this.enabled ? id : null;
    if (id === null) {
      if (this.inDvr) void this.toLive();
      // The VOD shows up a few seconds after the start.
      if (this.uptime < DVR_YOUNG) this.retry = setTimeout(() => void this.ask(), DVR_RETRY_MS);
    }
  }

  private flush(): void {
    const at = this.pending;
    this.pending = null;
    if (at === null) return;
    this.go(at);
  }

  private go(at: number | 'live'): void {
    if (at === 'live') return void this.toLive();
    const e = this.host.engine();
    // In the same source: a seek. From live or the other source: it opens there.
    if (this.inVod && e && e.replay === (this.mode === 'replay')) e.seekIn(at);
    else void this.host.open(at);
  }

  /** Seconds since the stream started for a date in ms. */
  private secs(ms: number): number {
    return (ms - this.origin) / 1000;
  }

  /** V108-N24 Reads what the replay holds, for the bar and the keys. */
  private async readRange(): Promise<void> {
    const id = this.host.liveId();
    if (this.replayMinutes <= 0 || this.vodId !== null || !id) {
      if (!id) this.range = null;
      return;
    }
    const n = ++this.ranges;
    const audio = this.host.engine()?.choice === 'audio';
    const r = await playerIpc.replayRange(id, audio).catch(() => null);
    if (n !== this.ranges) return;
    this.range = r ? { from: r.from, to: r.to, keep: r.keep } : null;
  }

  /** The chat bar's entry (Q23); LivePlayer also calls it when the player fails, so the bar goes at once. */
  mark(): void {
    if (this.inVod && !this.done) {
      const behind = Math.round(this.behind ?? 0);
      setRewound(this, this.login, { behind, ended: this.over });
    } else if (hasOwn(this)) {
      clearRewound(this);
    }
  }

  private tick(): void {
    // V107-H1 Hidden: the clock and the chat bar wait, what keeps the sound going still runs.
    const hidden = pageHidden();
    const up = !this.ended && this.startedAt ? uptimeAt(this.startedAt, Date.now()) : this.uptime;
    if (!hidden) this.uptime = up;
    const e = this.host.engine();
    if (!hidden) this.mark();
    if (!hidden && this.ticks++ % RANGE_EVERY_S === 0) void this.readRange();
    // The VOD's edge stalls: nothing more to load there yet, live has it.
    if (this.inVod && e && e.waiting && !e.swapping && !this.ended && up - e.time < 40) {
      this.stalls += 1;
      if (this.stalls >= 3) {
        this.stalls = 0;
        void this.toLive();
      }
    } else this.stalls = 0;
    // Q21 a After the VOD's end the usual offline page, 5 s later.
    if (this.over && e?.failure === 'offline' && !this.endTimer && !this.done) {
      this.endTimer = setTimeout(() => {
        this.done = true;
        clearRewound(this);
      }, 5000);
    }
  }
}
