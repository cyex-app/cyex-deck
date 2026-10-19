// hls.js behind one small class (1.1). Loaded on first use, the light build: no
// subtitles, DRM or alternate audio. Quality is chosen here, not by hls.js's bandwidth
// guess (see quality.ts). Audio only (P1) loads the session's audio master instead.

import type HlsType from 'hls.js';
import type { ErrorData, HlsConfig } from 'hls.js';

import type { PlayerSession } from '../../ipc/bindings/PlayerSession';
import { delayVideo } from './avsync';
import { onShown, pageHidden } from '../../lib/awake';
import { Away, outOfSight } from './away';
import { GRAPH_EVENT, graphDelay } from './gain';
import { behindLive, dvrConfig, liveConfig, liveState, maxLatency, PAUSE_STOP_MS, pausedBehind, rangeEnd } from './liveEdge';
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
import { stallStart, stallStep } from './stall';
import { isTorn, tornDue, tornStart } from './torn';

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
  /** d-4 Playing live, the picture hasn't moved for 10 s: it reopens every 10 s until it does. */
  stalled = $state(false);
  /** d-4 Set by LivePlayer: an ad break waits on purpose, that's no stall. */
  adBreak = false;
  /** Live players: reopens the stream, live when `at` is null, else the VOD from there. */
  onreopen: ((at: number | null) => void) | null = null;

  private hls: HlsType | null = null;
  private H: typeof HlsType | null = null;
  /** V1061-01 U11 Pause, jump and swap state (liveMachine.ts). */
  private st = $state.raw<LiveState>(initial());
  private stopTimer: ReturnType<typeof setTimeout> | undefined;
  /** d-1 Behind live at the user's pause, null when unknown or not live; ticks once a second while paused. */
  private behindAtPause: number | null = null;
  private pauseTick: ReturnType<typeof setInterval> | undefined;
  private stall = stallStart(0);
  private stallTimer: ReturnType<typeof setInterval> | undefined;
  private lastUpdate = 0;
  /** qd-2 Discontinuity (cc) of the fragment playing, null before one plays. */
  private playCc: number | null = null;
  /** qd-3 The live playlist's newest content (program date time, ms). */
  private edgeDate: number | null = null;
  /** h3 Live fragments loading: where the playlist puts them (sn → s). */
  private planned = new Map<number, number>();
  /** h3 The first fragment hls.js placed off its spot, and that spot; null when none. */
  private tornSn: number | null = null;
  private tornAt: number | null = null;
  private session: PlayerSession | null = null;
  /** VODs: where to begin, in seconds. */
  private startAt = -1;
  private playerPx = 720;
  private recovered = 0;
  /** rd-2 Counts loads: only the newest makes an hls instance. */
  private loads = 0;
  /** V103-03 */
  private readonly away = new Away(() => this.drop());
  /** The page hides the player: its tab plays in the background (V102-15). */
  private hidden = false;
  /** V107-H1 The window is minimized, in the tray or covered. */
  private pageHidden = pageHidden();
  private readonly off: () => void;
  /** K3 Stops listening for boost graph changes of the current hls. */
  private unGraph: () => void = () => {};
  private graphTimer: ReturnType<typeof setTimeout> | undefined;
  /** L1 Destroyed: nothing starts or loads on it again. */
  private gone = false;

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
        this.unhold();
        this.onPlay();
      },
      playing: () => {
        this.playing = true;
        this.waiting = false;
        this.st = landed(this.st);
      },
      pause: () => this.onPause(),
      waiting: () => {
        this.waiting = true;
        this.checkTorn();
      },
      canplay: () => this.onCanplay(),
      timeupdate: () => this.tick(),
      seeked: () => this.tick(),
      // Q21 a The rewind VOD ran out after the stream ended.
      ended: () => this.dvr && this.fail('offline'),
      enterpictureinpicture: () => this.sight(),
      leavepictureinpicture: () => this.sight(),
    };
    for (const [name, f] of Object.entries(set)) on(name, f);
    if (live) this.stallTimer = setInterval(() => this.checkStall(), 1000);
    const unShown = onShown((h) => {
      this.pageHidden = h;
      this.sight();
    });
    this.off = () => {
      for (const [name, f] of Object.entries(set)) video.removeEventListener(name, f);
      unShown();
    };
  }

  /** `hold`: paused until played (V103-07). `dvr`: the session is the rewind VOD (V1061-03). */
  async start(session: PlayerSession, choice: Choice, startAt = -1, hold = false, dvr = false): Promise<void> {
    // Q14 a A swap: silent, nothing plays meanwhile.
    if (this.gone) return;
    const swap = !!this.hls;
    this.st = begin(this.st, swap, dvr);
    if (swap) this.quietPause();
    this.dvr = dvr;
    clearTimeout(this.stopTimer);
    this.unhold();
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
    // qd-2 Waiting for hls.js's next reload (up to a target duration, 6 s on Twitch) made Play
    // slow. Only a playlist that stopped coming is waited for.
    const age = performance.now() - this.lastUpdate;
    if (age > STALE_PLAYLIST_MS && !(await this.levelUpdate(hls, 4000))) {
      if (this.hls === hls) this.onreopen?.(null);
      return;
    }
    // qd-2 A few seconds old, hls.js keeps the sync point inside the old playlist: it lands up to
    // 5 s short of live and the button reads -0:05 at the next reload. Asked for now, one round trip.
    if (age > FRESH_PLAYLIST_MS && reloadNow(hls)) await this.levelUpdate(hls, 1000);
    if (this.hls !== hls) return;
    const t = hls.liveSyncPosition;
    if (t === null) return void this.onreopen?.(null);
    const v = this.video;
    if (t - v.currentTime < 1) {
      hls.resumeBuffering();
      if (play) void v.play().catch(() => {});
      return;
    }
    // qd-2 Seek first, then buffer: resumed at the old spot, hls.js first fetched the segments
    // after the paused buffer, and the live ones waited behind them.
    if (rangeEnd(v.buffered, t) !== null) {
      v.currentTime = t;
      hls.resumeBuffering();
      if (play) void v.play().catch(() => {});
      return;
    }
    // qd-2 Live is past a discontinuity the buffer isn't in (an ad switch, a restart): hls.js
    // didn't place the new part, the seek waited 4 s and more. A new instance on this session
    // starts there like a fresh open.
    const at = hls.levels[hls.loadLevel]?.details?.fragments.find((f) => t >= f.start && t < f.start + f.duration);
    if (play && at && this.playCc !== null && at.cc !== this.playCc) {
      this.quietPause();
      return void this.load();
    }
    // Q14 a Silent while it loads there: no pop, no repeated seconds.
    this.st = jumpBegin(this.st);
    this.quietPause();
    v.currentTime = t;
    hls.resumeBuffering();
    const landed = await waitFor(v, 'canplay', JUMP_MS);
    this.st = jumpEnd(this.st);
    if (this.hls !== hls) return;
    const p = takePending(this.st);
    this.st = p.s;
    if (p.pending) this.markPaused();
    // qd-2 Still nothing there: the same, a new instance (the old wait was 4 s, then stuck).
    else if (!landed && play) void this.load();
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
    const moved = !this.hidden && Math.abs(px - this.playerPx) >= 40;
    if (moved) this.playerPx = px;
    if (!this.sight() && moved && this.choice === 'auto') this.apply(true);
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
    this.gone = true;
    clearTimeout(this.stopTimer);
    clearInterval(this.pauseTick);
    clearInterval(this.stallTimer);
    this.away.stop();
    this.off();
    this.unGraph();
    this.loads += 1;
    this.dropHls();
  }

  /** rd-2 The old instance goes for good: nothing of it loads on. */
  private dropHls(): void {
    this.hls?.destroy();
    this.hls = null;
  }

  private async load(): Promise<void> {
    const session = this.session;
    if (!session || this.gone) return;
    const n = ++this.loads;
    this.dropHls();
    this.failure = null;
    this.waiting = true;
    this.recovered = 0;
    this.playCc = null;
    this.planned.clear();
    this.tornAt = null;
    const Hls = await loadHls();
    // rd-2 A newer load (or start) came meanwhile: it makes the instance, or two would load.
    if (this.session !== session || n !== this.loads || this.gone) return;
    this.dropHls();
    const hls = new Hls(this.config());
    this.hls = hls;
    this.H = Hls;
    // K3 A boosted video's picture waits for its Web Audio sound. A new delay reloads the
    // buffer at the play head, so it's in sync at once. Read again when the graph changes,
    // until the context has its output latency (a moment after it starts).
    const check = delayVideo(hls, Hls.Events.BUFFER_APPENDING, () => graphDelay(this.video), () => this.resync(hls));
    this.unGraph();
    const onGraph = (tries = 25) => {
      clearTimeout(this.graphTimer);
      if (this.hls !== hls || check() || tries <= 0) return;
      this.graphTimer = setTimeout(() => onGraph(tries - 1), 200);
    };
    const graphed = () => onGraph();
    this.video.addEventListener(GRAPH_EVENT, graphed);
    this.unGraph = () => {
      clearTimeout(this.graphTimer);
      this.video.removeEventListener(GRAPH_EVENT, graphed);
    };
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
      // Paused during the swap, or reopened while paused (rd-2): it lands paused (Q14 a),
      // this instance's buffering paused too.
      if (this.st.pendingPause || isPaused(this.st)) {
        this.st = takePending(this.st).s;
        this.waiting = false;
        return this.markPaused(this.liveSrc ? undefined : this.startAt);
      }
      void this.video.play().catch(() => {});
    });
    hls.on(Hls.Events.LEVEL_SWITCHED, (_, d) => (this.level = d.level));
    // qd-2 The discontinuity the play head is in.
    hls.on(Hls.Events.FRAG_CHANGED, (_, d) => (this.playCc = d.frag.cc));
    hls.on(Hls.Events.LEVEL_UPDATED, (_, d) => {
      this.lastUpdate = performance.now();
      // qd-3 The newest content the playlist has (program date time), for the stall check.
      const last = d.details.fragments.at(-1);
      if (this.liveSrc && last?.programDateTime != null) this.edgeDate = last.programDateTime + last.duration * 1000;
      this.tick();
    });
    hls.on(Hls.Events.ERROR, (_, d) => this.onError(Hls, d));
    // h3 A clock jump hls.js got no discontinuity for (torn.ts).
    hls.on(Hls.Events.FRAG_LOADING, (_, d) => {
      if (!this.liveSrc || typeof d.frag.sn !== 'number') return;
      this.planned.set(d.frag.sn, d.frag.start);
      if (this.planned.size > 40) this.planned.delete(this.planned.keys().next().value!);
    });
    hls.on(Hls.Events.LEVEL_PTS_UPDATED, (_, d) => {
      const sn = d.frag.sn;
      if (!this.liveSrc || typeof sn !== 'number' || this.tornAt !== null || !isTorn(this.planned.get(sn), d.start)) return;
      this.tornSn = sn;
      this.tornAt = this.planned.get(sn)!;
      this.checkTorn();
    });
    // h3 The instance after a torn fragment starts there, not before: the old timeline would tear again.
    const tornSn = this.tornSn;
    if (tornSn !== null) {
      hls.once(Hls.Events.LEVEL_LOADED, (_, d) => {
        if (this.tornSn === tornSn) this.tornSn = null;
        const at = tornStart(d.details.fragments, tornSn, hls.liveSyncPosition);
        if (at !== null && this.hls === hls) hls.startLoad(at);
      });
    }
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

  /** V103-03 Out of sight is hidden on the page and not in picture in picture; V107-H1
   *  or the whole page hidden. True when the quality came back. */
  private sight(): boolean {
    if (outOfSight(this.hidden, this.pageHidden, document.pictureInPictureElement === this.video)) this.away.hide();
    else if (this.away.show()) {
      this.apply(true);
      return true;
    }
    return false;
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

  /** K3 The delay changed with video buffered: everything buffered goes and loading starts
   *  again at the play head, so nothing plays with the old delay and no hole is left. That's
   *  hls.js's immediate switch to the level it loads (levels are always set by hand here):
   *  it aborts the segment loading, flushes both buffers and refetches from the play head;
   *  the playlists go on. The element isn't paused or seeked (the U11 flags stay), live
   *  keeps its spot. A paused player loads one segment, then stops buffering again. False
   *  while loading is off (held, or stopped after a long pause): the next append resyncs. */
  private resync(hls: HlsType): boolean {
    const H = this.H;
    if (this.hls !== hls || !H || !hls.loadingEnabled || hls.loadLevel < 0) return false;
    const v = this.video;
    if (!hls.bufferingEnabled) {
      hls.resumeBuffering();
      // rd-2 Only a pause that is still on: a jump after the play pauses the element too, and
      // stopping its buffering there left the jump waiting for a canplay that never came.
      hls.once(H.Events.FRAG_BUFFERED, () => {
        if (this.hls === hls && v.paused && isPaused(this.st)) hls.pauseBuffering();
      });
    }
    hls.currentLevel = hls.loadLevel;
    return true;
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

  /** A pause that is the user's: the playlists keep loading, later nothing. `pos`: where it
   *  stands, when the element isn't there yet (a rewind VOD reopened while paused). */
  private markPaused(pos = this.video.currentTime): void {
    this.st = paused(this.st, performance.now(), pos);
    this.hls?.pauseBuffering();
    // d-1 No pull to the edge while paused, and the readout counts up from here.
    this.behindAtPause = this.liveSrc ? this.behind : null;
    this.edgePull(false);
    clearInterval(this.pauseTick);
    if (this.behindAtPause !== null) this.pauseTick = setInterval(() => this.tick(), 1000);
    clearTimeout(this.stopTimer);
    this.stopTimer = setTimeout(() => {
      this.hls?.stopLoad();
      this.st = timedOut(this.st);
    }, PAUSE_STOP_MS);
  }

  /** d-1 A play (or a new source): hls.js keeps it near the edge again, the readout follows the video. */
  private unhold(): void {
    clearInterval(this.pauseTick);
    this.behindAtPause = null;
    this.edgePull(true);
  }

  /** d-1 Whether hls.js may pull a lagging live video to the edge. */
  private edgePull(on: boolean): void {
    if (this.hls && this.liveSrc) this.hls.config.liveMaxLatencyDuration = maxLatency(this.lowLatency, !on);
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

  /** h3 The play head reached a torn fragment: a new instance on this session plays on from
   *  there, like a fresh open (the old one would freeze, then jump back). */
  private checkTorn(): void {
    const v = this.video;
    if (!this.liveSrc || !this.hls || v.paused || !tornDue(this.tornAt, v.currentTime)) return;
    this.tornAt = null;
    this.quietPause();
    void this.load();
  }

  /** V1061-01..03 Where the player is and how far behind live. */
  private tick(): void {
    const v = this.video;
    this.checkTorn();
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
    const held = this.behindAtPause !== null && v.paused && isPaused(this.st);
    this.behind = held ? pausedBehind(this.behindAtPause, performance.now() - this.st.pausedAt) : behindLive(this.sync, v.currentTime);
    this.atLive = liveState(this.atLive ? 'live' : 'behind', this.behind) === 'live';
  }

  /** d-4 Once a second while live. */
  private checkStall(): void {
    const v = this.video;
    const active = !!this.session && this.liveSrc && !isPaused(this.st) && !this.st.pendingPause && !this.holding && !this.failure && !this.adBreak;
    const ready = !v.paused && !v.seeking && v.readyState >= 3;
    const date = this.hls?.playingDate?.getTime() ?? null;
    const r = stallStep(this.stall, { now: performance.now(), pos: v.currentTime, active, ready, date, edge: this.edgeDate });
    this.stall = r.s;
    if (r.action === 'clear') this.stalled = false;
    if (r.action !== 'retry') return;
    this.stalled = true;
    this.onreopen?.(null);
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

/** qd-2 A live playlist older than this is waited for before going live (Twitch reloads every 2-6 s). */
const STALE_PLAYLIST_MS = 15_000;
/** qd-2 Older than this, the sync point falls short of live (sync 8 s − target duration 6 s). */
const FRESH_PLAYLIST_MS = 1500;

/** qd-2 Reloads the playing quality's live playlist now, not at hls.js's next turn (a paused
 *  player's comes once a target duration, 6 s on Twitch). hls.js has no public call for it:
 *  false when its insides changed, then the old playlist does. */
function reloadNow(hls: HlsType): boolean {
  const lc = (hls as unknown as { levelController?: { loadPlaylist?: () => void } }).levelController;
  const details = hls.levels[hls.loadLevel]?.details;
  if (typeof lc?.loadPlaylist !== 'function' || !details?.live) return false;
  details.requestScheduled = 0;
  lc.loadPlaylist();
  return true;
}

/** qd-2 How long a jump to live may load before a new instance takes over. */
const JUMP_MS = 1500;

/** Resolves true on `ev`, false after `ms`, the listener removed either way. */
function waitFor(el: EventTarget, ev: string, ms: number): Promise<boolean> {
  return new Promise((done) => {
    const f = (came: boolean) => {
      clearTimeout(t);
      el.removeEventListener(ev, on);
      done(came);
    };
    const on = () => f(true);
    const t = setTimeout(() => f(false), ms);
    el.addEventListener(ev, on);
  });
}
