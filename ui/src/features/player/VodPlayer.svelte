<!-- A VOD (1.1). Resumes where it was left (P5), previews along the timeline (P6), and
     tells its tile where it is so the chat can replay (P8). V107-Mv Small in a corner while
     the channel's About or Clips shows, paused while a clip plays. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { PlayerSession } from '../../ipc/bindings/PlayerSession';
  import type { Storyboard } from '../../ipc/bindings/Storyboard';
  import type { Tile } from '../../ipc/bindings/Tile';
  import type { Video } from '../../ipc/bindings/Video';
  import { playerIpc } from '../../ipc/player';
  import { formatDate } from '../../lib/format';
  import { bindingOf } from '../../lib/hotkeys.svelte';
  import { latest } from '../../lib/latest';
  import { settings } from '../../lib/settings.svelte';
  import { Engine } from './engine.svelte';
  import { playerMenu } from './menu';
  import { playerFullscreen, type TileState } from './players.svelte';
  import { qualityOptions } from './quality';
  import SeekBar from './SeekBar.svelte';
  import GearButton from './GearButton.svelte';
  import Shell from './Shell.svelte';
  import { useSound } from './sound.svelte';
  import Stats from './Stats.svelte';
  import { clock, linkTime } from './time';
  import { watchPip, copyText, openLink, pipFor, playerKeys } from './tools';
  import Volume from './Volume.svelte';
  import Loading from '../../components/Loading.svelte';
  import { keyTaken } from '../../lib/escape';
  import PipNotice from './PipNotice.svelte';

  interface Props {
    id: string;
    /** Seconds to start at; 0 resumes where it was left. */
    at: number;
    tile: Tile | undefined;
    account: string | null;
    shared?: TileState;
    hushed?: boolean;
    onclose?: () => void;
    /** V107-Mv Small in a corner, over another page. */
    mini?: boolean;
    /** V107-Mv Paused for a clip in front. */
    held?: boolean;
    /** V107-Mv Back to full size. */
    onexpand?: () => void;
  }

  let { id, at, tile, account, shared, hushed = false, onclose, mini = false, held = false, onexpand }: Props = $props();

  /** Escape closes it (not in fullscreen, where Escape leaves that first; not while a menu or
   *  dialog wants it or a text field has the keyboard). */
  function onkeydown(e: KeyboardEvent) {
    if (!onclose || mini || e.key !== 'Escape' || keyTaken(e) || document.fullscreenElement) return;
    if ((e.target as HTMLElement | null)?.closest?.('input, textarea, select, [contenteditable]')) return;
    e.preventDefault();
    onclose();
  }


  let video = $state<HTMLVideoElement>();
  let shell = $state<Shell>();
  let engine = $state<Engine>();
  let session: PlayerSession | null = null;
  let info = $state<Video | null>(null);
  let board = $state<Storyboard | null>(null);
  let time = $state(0);
  let duration = $state(0);
  let buffered = $state(0);
  let speed = $state(1);
  let stats = $state(false);
  let saved = 0;
  /** V107-Mv The engine is going: its reset to 0 must not overwrite the saved spot. */
  let gone = false;

  // svelte-ignore state_referenced_locally
  const sound = useSound(() => tile, () => hushed, true);

  $effect(() => {
    if (!video) return;
    gone = false;
    const e = new Engine(video, false, false);
    engine = e;
    info = null;
    if (session) void playerIpc.close(session.id);
    session = null;
    const current = starts.begin();
    void (async () => {
      try {
        const [s, last, meta] = await Promise.all([
          playerIpc.vod(id, account),
          at > 0 ? Promise.resolve(at) : playerIpc.position(id).catch(() => null),
          playerIpc.video(id).catch(() => null),
        ]);
        // Another video, or this one closed, while the session was made.
        if (!current()) return void playerIpc.close(s.id);
        session = s;
        info = meta;
        // v1.07 No saved spot: the start. -1 put a VOD still recording at its live end.
        await e.start(s, settings.v.player.quality, last ?? 0);
      } catch {
        if (current()) e.failure = 'media';
      }
    })();
    const ro = new ResizeObserver(([entry]) => e.fit(entry.contentRect.height * devicePixelRatio));
    ro.observe(video);
    return () => {
      ro.disconnect();
      starts.cancel();
      save();
      gone = true;
      e.destroy();
    };
  });

  $effect(() => {
    if (video) sound.apply(video);
  });

  $effect(() => {
    if (held) video?.pause();
  });
  $effect(() => video && sound.attach(video));

  $effect(() => {
    // The default too: a reload (quality, audio only, V101-17) would reset it.
    if (video) video.playbackRate = video.defaultPlaybackRate = speed;
  });

  onDestroy(() => {
    starts.cancel();
    save();
    if (shared?.vod?.id === id) shared.vod = null;
    if (session) void playerIpc.close(session.id);
  });

  function ontimeupdate() {
    if (!video || gone) return;
    time = video.currentTime;
    duration = Number.isFinite(video.duration) ? video.duration : info?.length ?? 0;
    const b = video.buffered;
    buffered = b.length ? b.end(b.length - 1) : 0;
    if (shared) shared.vod = { id, time };
    if (Math.abs(time - saved) > 15) save();
  }

  /** P5 */
  function save() {
    if (!video || !duration || gone) return;
    saved = video.currentTime;
    void playerIpc.savePosition(id, saved, duration);
  }

  function seek(t: number) {
    if (video) video.currentTime = Math.max(0, duration ? Math.min(t, duration) : t);
  }

  const starts = latest();

  /** A fresh session, carrying on where it was. */
  async function reload() {
    const e = engine;
    if (!e || !video) return;
    const at = video.currentTime;
    const current = starts.begin();
    if (session) void playerIpc.close(session.id);
    session = null;
    e.failure = null;
    e.waiting = true;
    try {
      const s = await playerIpc.vod(id, account);
      if (!current() || engine !== e) return void playerIpc.close(s.id);
      session = s;
      await e.start(s, e.choice, at);
    } catch {
      if (current()) e.failure = 'media';
    }
  }

  /** The video plays in its own small window (picture in picture). */
  let pip = $state(false);
  $effect(() => (video ? watchPip(video, (on) => (pip = on)) : undefined));

  const controls = {
    togglePlay: () => video && (video.paused ? void video.play().catch(() => {}) : video.pause()),
    toggleMute: () => sound.toggleMute(),
    // Small, a double-click brings it back instead.
    toggleFullscreen: () => (mini ? onexpand?.() : shell?.fullscreen()),
    play: () => void video?.play().catch(() => {}),
    pause: () => video?.pause(),
    seekBy: (s: number) => video && seek(video.currentTime + s),
    media: () => ({ title: info?.title || 'Video', artist: info?.channelName || tile?.channel.name || '', artwork: info?.thumbnail }),
  };

  /** V101-17 */
  function menu() {
    const e = engine;
    const link = `https://www.twitch.tv/videos/${id}`;
    return playerMenu({
      kind: 'vod',
      playing: Boolean(e?.playing),
      muted: sound.muted,
      fullscreen: Boolean(document.fullscreenElement),
      keys: playerKeys(),
      quality: e
        ? qualityOptions(e.levels, e.level, e.choice).map((o) => ({ ...o, on: o.choice === e.choice, pick: () => void e.choose(o.choice) }))
        : [],
      togglePlay: controls.togglePlay,
      toggleMute: controls.toggleMute,
      toggleFullscreen: () => {
        if (mini) onexpand?.();
        shell?.fullscreen();
      },
      speed: { now: speed, set: (s) => (speed = s) },
      pip: pipFor(video),
      stats: { on: stats, toggle: () => (stats = !stats) },
      reload: () => void reload(),
      link,
      linkAt: `${link}?t=${linkTime(video?.currentTime ?? 0)}`,
      copy: copyText,
      open: openLink,
    });
  }

  const failure = $derived(engine?.failure);
  const corner = $derived(settings.v.player.miniCorner ?? 'bottomRight');
  const key = (k: Parameters<typeof bindingOf>[0], label: string) => (bindingOf(k) ? `${label} (${bindingOf(k)})` : label);
</script>

<svelte:window {onkeydown} />

<div class="vod {mini ? corner : ''}" class:mini class:held>
  <Shell bind:this={shell} {controls} {menu} pinned={!engine?.playing && !mini} onvolume={(s) => sound.step(s)}>
    <video
      bind:this={video}
      playsinline
      {ontimeupdate}
      ondurationchange={ontimeupdate}
    ></video>
    {#if pip}<PipNotice name={info?.channelName || tile?.channel.name || 'The video'} avatar={null} />{/if}
    {#if failure}
      <div class="center">
        {failure === 'subs'
          ? 'This video is for subscribers only.'
          : failure === 'gone'
            ? 'This video is gone.'
            : failure === 'geo'
              ? 'Not available in your country.'
              : 'The video didn’t load.'}
        {#if failure !== 'subs' && failure !== 'gone' && failure !== 'geo'}
          <button class="retry" onclick={() => void reload()}><Icon name="refresh" size={14} />Try again</button>
        {/if}
      </div>
    {:else if engine?.waiting}
      <div class="busy"><Loading video size="large" /></div>
    {/if}
    {#if stats && engine}<Stats read={() => engine!.stats()} {video} onclose={() => (stats = false)} />{/if}
    {#if mini}
      {#if engine && !failure}
        <div class="mini-play">
          <button class="ctl" aria-label={engine.playing ? 'Pause' : 'Play'} onclick={controls.togglePlay}>
            <Icon name={engine.playing ? 'pause' : 'play'} size={26} />
          </button>
        </div>
      {/if}
      <div class="mini-bar">
        <button class="ctl" aria-label="Back to the video" use:tooltip={'Back to the video'} onclick={onexpand}><Icon name="expand" size={14} /></button>
        <button class="ctl" aria-label="Close" use:tooltip={'Close the video'} onclick={onclose}><Icon name="x" size={14} /></button>
      </div>
    {/if}

    {#snippet top()}
      {#if !mini}
        <div class="title">
          <b>{info?.title || 'Video'}</b>
          {#if info}<span>{[info.game, formatDate(new Date(info.publishedAt))].filter(Boolean).join(' · ')}</span>{/if}
        </div>
        {#if onclose}
          <button class="ctl" aria-label="Close" use:tooltip={'Close the video'} onclick={onclose}><Icon name="x" size={16} /></button>
        {/if}
      {/if}
    {/snippet}

    {#snippet bar()}
      {#if !mini}
        <div class="rows">
          <SeekBar {duration} {time} {buffered} storyboard={board} onwantpreview={() => void playerIpc.storyboard(id).then((b) => (board = b)).catch(() => {})} onseek={seek} />
          <div class="row">
            <button class="ctl" aria-label={engine?.playing ? 'Pause' : 'Play'} use:tooltip={key('player.playPause', engine?.playing ? 'Pause' : 'Play')} onclick={controls.togglePlay}>
              <Icon name={engine?.playing ? 'pause' : 'play'} size={18} />
            </button>
            <Volume volume={sound.volume} muted={sound.muted} boost={sound.boost} onchange={sound.set} onboost={sound.setBoost} />
            <span class="clock">{clock(time)} / {clock(duration)}</span>
            <span class="grow"></span>
            {#if engine}<GearButton {menu} />{/if}
            <button class="ctl" aria-label={playerFullscreen.on ? 'Exit fullscreen' : 'Fullscreen'} use:tooltip={key('player.fullscreen', playerFullscreen.on ? 'Exit fullscreen' : 'Fullscreen')} onclick={controls.toggleFullscreen}>
              <Icon name="expand" size={16} />
            </button>
          </div>
        </div>
      {/if}
    {/snippet}
  </Shell>
</div>

<style>
  .vod {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  /* V107-Mv Like the small stream (P4), in the same corner. */
  .mini {
    position: absolute;
    z-index: var(--z-panel);
    width: min(320px, 45%);
    aspect-ratio: 16 / 9;
    flex: none;
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
  }
  .topLeft {
    top: 12px;
    left: 12px;
  }
  .topRight {
    top: 12px;
    right: 12px;
  }
  .bottomLeft {
    bottom: 12px;
    left: 12px;
  }
  .bottomRight {
    bottom: 12px;
    right: 12px;
  }
  /* Over a clip: smaller, clear of its title and controls. */
  .held {
    width: min(220px, 35%);
  }
  .held.topLeft,
  .held.topRight {
    top: 52px;
  }
  .held.bottomLeft,
  .held.bottomRight {
    bottom: 64px;
  }
  .mini-play {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: grid;
    place-items: center;
    background: var(--scrim);
    opacity: 0;
    transition: opacity var(--base) var(--ease);
    pointer-events: none;
  }
  .mini:hover .mini-play,
  .mini-play:focus-within {
    opacity: 1;
  }
  .mini:hover .mini-play .ctl {
    pointer-events: auto;
  }
  .mini-play .ctl {
    width: 48px;
    height: 48px;
    pointer-events: none;
  }
  .mini-bar {
    position: absolute;
    top: 4px;
    right: 4px;
    z-index: 3;
    display: flex;
    gap: 2px;
    background: var(--scrim);
    border-radius: var(--radius);
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 16px;
    text-align: center;
    color: var(--on-video-dim);
    background: var(--scrim);
  }
  .retry {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
    padding: 6px 12px;
    background: color-mix(in srgb, var(--on-video) 14%, transparent);
    border: 0;
    border-radius: var(--radius);
    color: var(--on-video);
  }
  .busy {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title b,
  .title span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title span {
    font-size: 0.8rem;
    opacity: 0.85;
  }
  .rows {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 0 4px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .clock {
    margin-left: 4px;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .grow {
    flex: 1;
  }
</style>
