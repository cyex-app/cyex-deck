// hls.js behind one small class (1.1). Loaded on first use, the light build: no
// subtitles, DRM or alternate audio. Quality is chosen here, not by hls.js's bandwidth
// guess (see quality.ts). Audio only (P1) loads the session's audio master instead.

import type HlsType from 'hls.js';
import type { ErrorData, HlsConfig } from 'hls.js';

import type { PlayerSession } from '../../ipc/bindings/PlayerSession';
import { delayVideo } from './avsync';
import { Away } from './away';
import { graphDelay } from './gain';
import { behindLive, dvrConfig, liveConfig, liveState, PAUSE_STOP_MS, rangeEnd } from './liveEdge';
import {
  askPause,
  begin,
  brokeWhilePaused,
  canplayCheck,
  failed,
  initial,
  isPaused,
  isSwapping,
  jumpBegin,
  jumpEnd,
  landed,
  type LiveState,
  ownPaused,
  paused,
  played,
  playedWhilePaused,
  resumeStep,
  takePending,
  timedOut,
  toLiveStep,
  userPause,
} from './liveMachine';
import { type Choice, levelFor, levelName, lowestLevel } from './quality';

let hlsModule: Promise<typeof HlsType> | null = null;

function loadHls(): Promise<typeof HlsType> {
  hlsModule ??= import('hls.js/light').then((m) => m.default as unknown as typeof HlsType);
  return hlsModule;
}

/** Why playback stopped, from the proxy's answer. */
export type Failure = 'offline' | 'gone' | 'geo' | 'subs' | 'network' | 'media';

export interface Level {
  index: number;
  name: string;
  height: number;
}

export interface Stats {
  resolution: string;
  level: string;
  bitrate: number;
  bandwidth: number;
  latency: number | null;
  buffer: number;
  dropped: string;
}

export class Engine {
  levels = $state.raw<Level[]>([]);
  /** Level playing now, -1 before the first. */
  level = $state(-1);
  choice = $state<Choice>('auto');
  playing = $state(false);
  waiting = $state(true);
  failure = $state<Failure | null>(null);
  /** V103-07 Started paused: the levels are read, nothing loads until it plays. */
  holding = $state(false);
  /** V1061-03 The source is the broadcast's VOD (Stream Rewind), not the live playlist. */
  dvr = $state(false);
  /** V1061-01 Seconds behind the live point on the live source, null when unknown or rewound. */
  behind = $state<number | null>(null);
  atLive = $state(true);
  sync = $state<number | null>(null);
  /** `video.currentTime`, for the bar. */
  time = $state(0);
  /** End of the buffered range that holds `time`. */
  bufferedEnd = $state(0);
  /** A short readout for the player, taken and cleared by LivePlayer. */
  note = $state<string | null>(null);
  /** Live players: reopens the stream, live when `at` is null, else the VOD from there. */
  onreopen: ((at: number | null) => void) | null = null;

  private hls: HlsType | null = null;
  private H: typeof HlsType | null = null;
  /** V1061-01 U11 Pause, jump and swap state (liveMachine.ts). */
  private st = $state.raw<LiveState>(initial());
  private stopTimer: ReturnType<typeof setTimeout> | undefined;
  private lastUpdate = 0;
  private session: PlayerSession | null = null;
  /** VODs: where to begin, in seconds. */
  private startAt = -1;
  private playerPx = 720;
  private recovered = 0;
  /** V103-03 */
  private readonly away = new Away(() => this.drop());
  /** The page hides the player: its tab plays in the background (V102-15). */
  private hidden = false;
  private readonly off: () => void;

  constructor(
    private readonly video: HTMLVideoElement,
    private readonly live: boolean,
    private readonly lowLatency: boolean,
  ) {
    const on = (name: string, f: () => void) => video.addEventListener(name, f);
    const set = {
      play: () => {
        // Any own pause event came first or was dropped.
        this.st = played(this.st);
        this.onPlay();
      },
      playing: () => {
        this.playing = true;
        this.waiting = false;
        this.st = landed(this.st);
      },
      pause: () => this.onPause(),
      waiting: () => (this.waiting = true),
      canplay: () => this.onCanplay(),
      timeupdate: () => this.tick(),
      seeked: () => this.tick(),
      // Q21 a The rewind VOD ran out after the stream ended.
      ended: () => this.dvr && this.fail('offline'),
      enterpictureinpicture: () => this.sight(),
      leavepictureinpicture: () => this.sight(),
    };
    for (const [name, f] of Object.entries(set)) on(name, f);
    this.off = () => {
      for (const [name, f] of Object.entries(set)) video.removeEventListener(name, f);
    };
  }

  /** `hold`: paused until played (V103-07). `dvr`: the session is the rewind VOD (V1061-03). */
  async start(session: PlayerSession, choice: Choice, startAt = -1, hold = false, dvr = false): Promise<void> {
    // Q14 a A swap: silent, nothing plays meanwhile.
    const swap = !!this.hls;
    this.st = begin(this.st, swap, dvr);
    if (swap) this.quietPause();
    this.dvr = dvr;
    clearTimeout(this.stopTimer);
    if (dvr) this.time = startAt;
    this.behind = null;
    this.sync = null;
    this.atLive = !dvr;
    this.session = session;
    this.choice = choice;
    this.startAt = startAt;
    this.holding = hold;
    await this.load();
  }

  get paused(): boolean {
    return this.video.paused;
  }

  /** V1061-03 Q14 a One source swaps for the other: no spinner unless it takes long. */
  get swapping(): boolean {
    return isSwapping(this.st);
  }

  /** A deliberate pause, e.g. a clip over the rewound stream (Q22 b). */
  pause(): void {
    this.st = askPause(this.st);
    this.video.pause();
  }

  /** V1061-01 Play: a live player decides where it goes on (LivePlayer's play paths). */
  async resume(): Promise<void> {
    const r = resumeStep(this.st, { live: this.live, holding: this.holding, dvr: this.dvr, hasHls: !!this.hls, now: performance.now() });
    this.st = r.s;
    // Played again mid-jump after a pause: the jump plays when it lands.
    if (r.step === 'wait') return;
    if (r.step === 'direct') {
      void this.video.play().catch(() => {});
      return;
    }
    clearTimeout(this.stopTimer);
    const plan = r.step;
    const pos = r.pos;
    if (plan === 'play' || plan === 'stay') {
      this.hls?.resumeBuffering();
      void this.video.play().catch(() => {});
    } else if (plan === 'sync') {
      await this.toLive(true);
    } else if (plan === 'reload') {
      if (this.hls) {
        this.hls.startLoad(pos);
        void this.video.play().catch(() => {});
      } else this.onreopen?.(pos);
    } else {
      this.onreopen?.(null);
    }
  }

  /** V1061-01/02/05 To the live point of this session, playing when asked. The rewind VOD,
   *  a stopped or missing session reopen the live stream instead. */
  async toLive(play = true): Promise<void> {
    const r = toLiveStep(this.st);
    if (!r) return;
    clearTimeout(this.stopTimer);
    this.st = r.s;
    const hls = this.hls;
    if (this.dvr || !hls || r.stopped) {
      this.onreopen?.(null);
      return;
    }
    hls.resumeBuffering();
    if (performance.now() - this.lastUpdate > 3000 && !(await this.levelUpdate(hls, 4000))) {
      if (this.hls === hls) this.onreopen?.(null);
      return;
    }
    if (this.hls !== hls) return;
    const t = hls.liveSyncPosition;
    if (t === null) return void this.onreopen?.(null);
    const v = this.video;
    if (t - v.currentTime < 1) {
      if (play) void v.play().catch(() => {});
      return;
    }
    if (rangeEnd(v.buffered, t) !== null) {
      v.currentTime = t;
      if (play) void v.play().catch(() => {});
      return;
    }
    // Q14 a Silent while it loads there: no pop, no repeated seconds.
    this.st = jumpBegin(this.st);
    this.quietPause();
    v.currentTime = t;
    await waitFor(v, 'canplay', 4000);
    this.st = jumpEnd(this.st);
    if (this.hls !== hls) return;
    const p = takePending(this.st);
    this.st = p.s;
    if (p.pending) this.markPaused();
    else if (play) await v.play().catch(() => {});
  }

  /** V1061-03 A seek inside the rewind VOD. */
  seekIn(t: number): void {
    if (!this.dvr) return;
    this.video.currentTime = t;
    this.time = t;
  }

  /** P1 Switching to or from audio only reloads; other choices switch at the next segment. */
  async choose(choice: Choice): Promise<void> {
    const reload = (choice === 'audio') !== (this.choice === 'audio');
    this.choice = choice;
    if (!reload) return this.apply(true);
    // A VOD carries on where it was.
    if (!this.liveSrc) this.startAt = this.video.currentTime;
    await this.load();
  }

  /** The player's height on screen, in device pixels: "auto" picks by it. 0 is out of
   *  sight: the quality stays for a minute, then drops until it shows again (V103-03). */
  fit(px: number): void {
    this.hidden = px <= 0;
    if (this.hidden) return this.sight();
    const moved = Math.abs(px - this.playerPx) >= 40;
    if (moved) this.playerPx = px;
    if (this.away.show() || (moved && this.choice === 'auto')) this.apply(true);
  }

  stats(): Stats {
    const v = this.video;
    const hls = this.hls;
    const lvl = hls?.levels[hls.currentLevel];
    const q = v.getVideoPlaybackQuality?.();
    const buffer = (rangeEnd(v.buffered, v.currentTime) ?? v.currentTime) - v.currentTime;
    return {
      resolution: v.videoWidth ? `${v.videoWidth}×${v.videoHeight}` : 'audio',
      level: lvl ? (lvl.height ? levelName(lvl) : 'audio') : '',
      bitrate: lvl?.bitrate ?? 0,
      bandwidth: hls?.bandwidthEstimate ?? 0,
      latency: this.liveSrc && hls ? hls.latency : null,
      buffer,
      dropped: q ? `${q.droppedVideoFrames} / ${q.totalVideoFrames}` : '',
    };
  }

  destroy(): void {
    clearTimeout(this.stopTimer);
    this.away.stop();
    this.off();
    this.hls?.destroy();
    this.hls = null;
  }

  private async load(): Promise<void> {
    const session = this.session;
    if (!session) return;
    this.hls?.destroy();
    this.failure = null;
    this.waiting = true;
    this.recovered = 0;
    const Hls = await loadHls();
    if (this.session !== session) return;
    const hls = new Hls(this.config());
    this.hls = hls;
    this.H = Hls;
    // K3 A boosted video's picture waits for its Web Audio sound. A new delay refetches what's
    // ahead (a smooth switch to the level loading), so it lands within a segment.
    delayVideo(hls, Hls.Events.BUFFER_APPENDING, () => graphDelay(this.video), () => {
      if (this.hls === hls && hls.loadLevel >= 0) hls.nextLevel = hls.loadLevel;
    });
    hls.on(Hls.Events.MANIFEST_PARSED, () => {
      this.levels = hls.levels
        .map((l, index) => ({ index, name: l.height ? levelName(l) : 'Audio only', height: l.height }))
        .sort((a, b) => b.height - a.height);
      this.apply(false);
      if (this.holding) {
        this.waiting = false;
        return;
      }
      // With autoStartLoad off hls.js skips `startPosition`: a VOD would start at 0 (P5, P7).
      hls.startLoad(this.liveSrc ? -1 : this.startAt);
      // Paused during the swap: it lands paused (Q14 a).
      if (this.st.pendingPause) {
        this.st = takePending(this.st).s;
        this.waiting = false;
        return this.markPaused();
      }
      void this.video.play().catch(() => {});
    });
    hls.on(Hls.Events.LEVEL_SWITCHED, (_, d) => (this.level = d.level));
    hls.on(Hls.Events.LEVEL_UPDATED, () => {
      this.lastUpdate = performance.now();
      this.tick();
    });
    hls.on(Hls.Events.ERROR, (_, d) => this.onError(Hls, d));
    hls.loadSource(this.choice === 'audio' ? session.audio : session.url);
    hls.attachMedia(this.video);
  }

  private config(): Partial<HlsConfig> {
    // Buffers stay small: every second ahead or behind is memory (D13).
    const common: Partial<HlsConfig> = {
      enableWorker: true,
      autoStartLoad: false,
      capLevelToPlayerSize: false,
      maxBufferSize: 30 * 1000 * 1000,
      // Nothing reads metadata cues (ads are found in Rust). Their track's `data:` source
      // broke the CSP's media-src with an error for every stream.
      enableID3MetadataCues: false,
      enableEmsgMetadataCues: false,
      enableDateRangeMetadataCues: false,
    };
    if (!this.live) {
      return { ...common, backBufferLength: 20, maxBufferLength: 30, maxMaxBufferLength: 40, startPosition: this.startAt };
    }
    // V1061-03 The rewind VOD is an EVENT playlist: hls.js must not pull it to its edge.
    if (this.dvr) return { ...common, lowLatencyMode: false, ...dvrConfig(this.startAt) };
    // V1061-01 No catch-up rate: it does nothing without low latency mode (Q14 a).
    return { ...common, lowLatencyMode: false, ...liveConfig(this.lowLatency) };
  }

  private apply(smooth: boolean): void {
    const hls = this.hls;
    if (!hls || this.choice === 'audio') return;
    const i = levelFor(hls.levels, this.choice, this.playerPx, this.away.low);
    if (i < 0) return;
    if (!smooth) {
      // A manual level keeps hls.js's own switching off.
      hls.startLevel = i;
      hls.loadLevel = i;
    } else if (i !== hls.loadLevel) {
      this.fresh(i);
      hls.nextLevel = i;
    }
  }

  /** V103-03 A live quality switched to is read afresh. hls.js keeps the playlist of a
   *  quality it left, and our proxy numbers each quality's segments only while it's
   *  served, so a quality back after a while looked as if no time had passed: hls.js put
   *  the stream minutes off and jumped there. Read afresh, it's lined up with the one
   *  playing by the segments' times. */
  private fresh(i: number): void {
    const hls = this.hls;
    if (!this.liveSrc || !hls || i === hls.currentLevel) return;
    const level = hls.levels[i];
    if (level) level.details = undefined;
  }

  /** V103-07 Played while held: loading starts now. Before the levels are read, they
   *  start it themselves. */
  private release(): void {
    if (!this.holding) return;
    this.holding = false;
    if (!this.hls?.levels.length) return;
    this.waiting = true;
    this.hls.startLoad(this.liveSrc ? -1 : this.startAt);
  }

  /** V103-03 Out of sight is hidden on the page and not in picture in picture. */
  private sight(): void {
    if (this.hidden && document.pictureInPictureElement !== this.video) this.away.hide();
    else if (this.away.show()) this.apply(true);
  }

  /** V103-03 A minute out of sight: the lowest quality from the next segment on. What's
   *  loaded already plays out, nobody sees it. Audio only stays as it is. */
  private drop(): void {
    const hls = this.hls;
    if (!hls || this.choice === 'audio') return;
    const i = lowestLevel(hls.levels);
    if (i < 0 || i === hls.loadLevel) return;
    this.fresh(i);
    hls.loadLevel = i;
  }

  private onError(Hls: typeof HlsType, d: ErrorData): void {
    if (!d.fatal) return;
    const code = d.response?.code ?? 0;
    const body = String((d.networkDetails as XMLHttpRequest | undefined)?.responseText ?? '');
    if (d.type === Hls.ErrorTypes.NETWORK_ERROR && this.live && isPaused(this.st)) {
      // V1061-01 The session went away while paused (Rust sweeps unpolled ones): play reopens quietly.
      this.st = brokeWhilePaused(this.st);
      this.hls?.destroy();
      this.hls = null;
    } else if (d.type === Hls.ErrorTypes.NETWORK_ERROR && this.dvr) {
      // Q15 a That part of the rewind VOD is gone.
      this.note = 'That part isn’t available anymore. Back to live.';
      this.onreopen?.(null);
    } else if (d.type === Hls.ErrorTypes.NETWORK_ERROR) {
      // usher's offline 404 says `content_geoblocked_or_offline`: offline. A geo block is `content_geoblocked` alone.
      if (code === 404 && (body.includes('Can not find channel') || body.includes('_or_offline') || !this.live)) this.fail('offline');
      else if (/geoblocked(?!_or_offline)/.test(body)) this.fail('geo');
      else if (code === 403 && !this.live) this.fail('subs');
      else this.fail('network');
    } else if (d.type === Hls.ErrorTypes.MEDIA_ERROR && this.recovered < 2) {
      this.recovered += 1;
      this.hls?.recoverMediaError();
    } else {
      this.fail('media');
    }
  }

  private fail(f: Failure): void {
    this.failure = this.live || f !== 'offline' ? f : 'gone';
    this.waiting = false;
    this.st = failed(this.st);
    this.hls?.destroy();
    this.hls = null;
  }

  /** The live playlist plays (not the rewind VOD). */
  private get liveSrc(): boolean {
    return this.live && !this.dvr;
  }

  /** Pauses for our own reasons: the pause event isn't the user's. */
  private quietPause(): void {
    if (this.video.paused) return;
    this.st = ownPaused(this.st);
    this.video.pause();
  }

  /** V1061-01 Q16 a Paused live: only the playlists keep loading, 10 min later nothing. */
  private onPause(): void {
    this.playing = false;
    const r = userPause(this.st, { videoPaused: this.video.paused, live: this.live, holding: this.holding });
    this.st = r.s;
    // The user paused between a swap's play and its first frame: the swap ends paused. Left
    // waiting, `swapping` hid the behind readout and the jump blocked the way back to live.
    if (r.wasJumping) this.waiting = false;
    if (r.mark) this.markPaused();
  }

  /** A pause that is the user's: the playlists keep loading, later nothing. */
  private markPaused(): void {
    this.st = paused(this.st, performance.now(), this.video.currentTime);
    this.hls?.pauseBuffering();
    clearTimeout(this.stopTimer);
    this.stopTimer = setTimeout(() => {
      this.hls?.stopLoad();
      this.st = timedOut(this.st);
    }, PAUSE_STOP_MS);
  }

  /** Plays that skip `resume()` (picture in picture, media session) still go where it decides. */
  private onPlay(): void {
    if (this.holding) return this.release();
    const r = playedWhilePaused(this.st, { live: this.live, hasHls: !!this.hls, dvr: this.dvr, now: performance.now() });
    if (r.step === 'none') return;
    if (r.step === 'keep') {
      // 'stay' or 'play': it already plays where it should.
      clearTimeout(this.stopTimer);
      this.st = r.s;
      this.hls?.resumeBuffering();
      return;
    }
    this.quietPause();
    void this.resume();
  }

  private onCanplay(): void {
    this.waiting = false;
    const c = canplayCheck(this.st);
    this.st = c.s;
    if (!c.check) return;
    // Research s.7 #5: `startPosition` on an EVENT playlist isn't sure to hold.
    if (Math.abs(this.video.currentTime - this.startAt) > 2) this.video.currentTime = this.startAt;
  }

  /** V1061-01..03 Where the player is and how far behind live. */
  private tick(): void {
    const v = this.video;
    // A swap shows the asked-for spot until the new source plays; after a failure the last spot stays.
    if (!this.swapping && !this.failure) {
      this.time = v.currentTime;
      this.bufferedEnd = rangeEnd(v.buffered, v.currentTime) ?? 0;
    }
    const h = this.hls;
    if (!this.liveSrc || !h || this.swapping) {
      this.sync = null;
      this.behind = null;
      return;
    }
    this.sync = h.liveSyncPosition;
    this.behind = behindLive(this.sync, v.currentTime);
    this.atLive = liveState(this.atLive ? 'live' : 'behind', this.behind) === 'live';
  }

  /** Resolves true on the next playlist update, false after `ms`. */
  private levelUpdate(hls: HlsType, ms: number): Promise<boolean> {
    const H = this.H;
    if (!H) return Promise.resolve(false);
    return new Promise((done) => {
      const f = () => {
        clearTimeout(t);
        done(true);
      };
      const t = setTimeout(() => {
        hls.off(H.Events.LEVEL_UPDATED, f);
        done(false);
      }, ms);
      hls.once(H.Events.LEVEL_UPDATED, f);
    });
  }
}

/** Resolves on `ev` or after `ms`, the listener removed either way. */
function waitFor(el: EventTarget, ev: string, ms: number): Promise<void> {
  return new Promise((done) => {
    const f = () => {
      clearTimeout(t);
      el.removeEventListener(ev, f);
      done();
    };
    const t = setTimeout(f, ms);
    el.addEventListener(ev, f);
  });
}
