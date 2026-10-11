// hls.js behind one small class (1.1). Loaded on first use, the light build: no
// subtitles, DRM or alternate audio. Quality is chosen here, not by hls.js's bandwidth
// guess (see quality.ts). Audio only (P1) loads the session's audio master instead.

import type HlsType from 'hls.js';
import type { ErrorData, HlsConfig } from 'hls.js';

import type { PlayerSession } from '../../ipc/bindings/PlayerSession';
import { type Choice, levelName, pickLevel } from './quality';

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

  private hls: HlsType | null = null;
  private session: PlayerSession | null = null;
  /** VODs: where to begin, in seconds. */
  private startAt = -1;
  private playerPx = 720;
  private recovered = 0;
  private readonly off: () => void;

  constructor(
    private readonly video: HTMLVideoElement,
    private readonly live: boolean,
    private readonly lowLatency: boolean,
  ) {
    const on = (name: string, f: () => void) => video.addEventListener(name, f);
    const set = {
      playing: () => ((this.playing = true), (this.waiting = false)),
      pause: () => (this.playing = false),
      waiting: () => (this.waiting = true),
      canplay: () => (this.waiting = false),
    };
    for (const [name, f] of Object.entries(set)) on(name, f);
    this.off = () => {
      for (const [name, f] of Object.entries(set)) video.removeEventListener(name, f);
    };
  }

  async start(session: PlayerSession, choice: Choice, startAt = -1): Promise<void> {
    this.session = session;
    this.choice = choice;
    this.startAt = startAt;
    await this.load();
  }

  /** P1 Switching to or from audio only reloads; other choices switch at the next segment. */
  async choose(choice: Choice): Promise<void> {
    const reload = (choice === 'audio') !== (this.choice === 'audio');
    this.choice = choice;
    if (!reload) return this.apply(true);
    // A VOD carries on where it was.
    if (!this.live) this.startAt = this.video.currentTime;
    await this.load();
  }

  /** The player's height on screen, in device pixels: "auto" picks by it. */
  fit(px: number): void {
    if (Math.abs(px - this.playerPx) < 40) return;
    this.playerPx = px;
    if (this.choice === 'auto') this.apply(true);
  }

  stats(): Stats {
    const v = this.video;
    const hls = this.hls;
    const lvl = hls?.levels[hls.currentLevel];
    const q = v.getVideoPlaybackQuality?.();
    let buffer = 0;
    for (let i = 0; i < v.buffered.length; i++) {
      if (v.buffered.start(i) <= v.currentTime && v.currentTime <= v.buffered.end(i)) buffer = v.buffered.end(i) - v.currentTime;
    }
    return {
      resolution: v.videoWidth ? `${v.videoWidth}×${v.videoHeight}` : 'audio',
      level: lvl ? (lvl.height ? levelName(lvl) : 'audio') : '',
      bitrate: lvl?.bitrate ?? 0,
      bandwidth: hls?.bandwidthEstimate ?? 0,
      latency: this.live && hls ? hls.latency : null,
      buffer,
      dropped: q ? `${q.droppedVideoFrames} / ${q.totalVideoFrames}` : '',
    };
  }

  destroy(): void {
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
    hls.on(Hls.Events.MANIFEST_PARSED, () => {
      this.levels = hls.levels
        .map((l, index) => ({ index, name: l.height ? levelName(l) : 'Audio only', height: l.height }))
        .sort((a, b) => b.height - a.height);
      this.apply(false);
      // With autoStartLoad off hls.js skips `startPosition`: a VOD would start at 0 (P5, P7).
      hls.startLoad(this.live ? -1 : this.startAt);
      void this.video.play().catch(() => {});
    });
    hls.on(Hls.Events.LEVEL_SWITCHED, (_, d) => (this.level = d.level));
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
    };
    if (!this.live) {
      return { ...common, backBufferLength: 20, maxBufferLength: 30, maxMaxBufferLength: 40, startPosition: this.startAt };
    }
    return {
      ...common,
      lowLatencyMode: false,
      // Segments still being made sit at the end: start two from the end.
      liveSyncDuration: this.lowLatency ? 4 : 8,
      liveMaxLatencyDuration: this.lowLatency ? 12 : 20,
      maxLiveSyncPlaybackRate: 1.1,
      backBufferLength: 10,
      maxBufferLength: 12,
      maxMaxBufferLength: 20,
    };
  }

  private apply(smooth: boolean): void {
    const hls = this.hls;
    if (!hls || this.choice === 'audio') return;
    const i = pickLevel(hls.levels, this.choice, this.playerPx);
    if (i < 0) return;
    if (!smooth) {
      // A manual level keeps hls.js's own switching off.
      hls.startLevel = i;
      hls.loadLevel = i;
    } else if (i !== hls.loadLevel) {
      hls.nextLevel = i;
    }
  }

  private onError(Hls: typeof HlsType, d: ErrorData): void {
    if (!d.fatal) return;
    const code = d.response?.code ?? 0;
    const body = String((d.networkDetails as XMLHttpRequest | undefined)?.responseText ?? '');
    if (d.type === Hls.ErrorTypes.NETWORK_ERROR) {
      if (code === 404 && (body.includes('Can not find channel') || !this.live)) this.fail('offline');
      else if (body.includes('geoblock')) this.fail('geo');
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
    this.hls?.destroy();
    this.hls = null;
  }
}
