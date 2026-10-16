<!-- A VOD (1.1). Resumes where it was left (P5), previews along the timeline (P6), and
     tells its tile where it is so the chat can replay (P8). -->
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
  import { settings } from '../../lib/settings.svelte';
  import { Engine } from './engine.svelte';
  import { playerMenu } from './menu';
  import type { TileState } from './players.svelte';
  import { qualityOptions } from './quality';
  import SeekBar from './SeekBar.svelte';
  import SettingsMenu from './SettingsMenu.svelte';
  import Shell from './Shell.svelte';
  import { useSound } from './sound.svelte';
  import Stats from './Stats.svelte';
  import { clock, linkTime } from './time';
  import { copyText, openLink, pipFor, playerKeys } from './tools';
  import Volume from './Volume.svelte';

  interface Props {
    id: string;
    /** Seconds to start at; 0 resumes where it was left. */
    at: number;
    tile: Tile | undefined;
    account: string | null;
    shared?: TileState;
    hushed?: boolean;
    onclose?: () => void;
  }

  let { id, at, tile, account, shared, hushed = false, onclose }: Props = $props();

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

  // svelte-ignore state_referenced_locally
  const sound = useSound(() => tile, () => hushed, true);

  $effect(() => {
    if (!video) return;
    const e = new Engine(video, false, false);
    engine = e;
    void (async () => {
      const [s, last, meta] = await Promise.all([
        playerIpc.vod(id, account),
        at > 0 ? Promise.resolve(at) : playerIpc.position(id).catch(() => null),
        playerIpc.video(id).catch(() => null),
      ]);
      session = s;
      info = meta;
      await e.start(s, settings.v.player.quality, last ?? -1);
    })();
    const ro = new ResizeObserver(([entry]) => e.fit(entry.contentRect.height * devicePixelRatio));
    ro.observe(video);
    return () => {
      ro.disconnect();
      e.destroy();
    };
  });

  $effect(() => {
    if (video) sound.apply(video);
  });
  $effect(() => {
    const v = video;
    return () => v && sound.release(v);
  });

  $effect(() => {
    // The default too: a reload (quality, audio only, V101-17) would reset it.
    if (video) video.playbackRate = video.defaultPlaybackRate = speed;
  });

  onDestroy(() => {
    save();
    if (shared?.vod?.id === id) shared.vod = null;
    if (session) void playerIpc.close(session.id);
  });

  function ontimeupdate() {
    if (!video) return;
    time = video.currentTime;
    duration = Number.isFinite(video.duration) ? video.duration : info?.length ?? 0;
    const b = video.buffered;
    buffered = b.length ? b.end(b.length - 1) : 0;
    if (shared) shared.vod = { id, time };
    if (Math.abs(time - saved) > 15) save();
  }

  /** P5 */
  function save() {
    if (!video || !duration) return;
    saved = video.currentTime;
    void playerIpc.savePosition(id, saved, duration);
  }

  function seek(t: number) {
    if (video) video.currentTime = Math.max(0, duration ? Math.min(t, duration) : t);
  }

  /** A fresh session, carrying on where it was. */
  async function reload() {
    const e = engine;
    if (!e || !video) return;
    const at = video.currentTime;
    if (session) void playerIpc.close(session.id);
    session = await playerIpc.vod(id, account);
    await e.start(session, e.choice, at);
  }

  const controls = {
    togglePlay: () => video && (video.paused ? void video.play() : video.pause()),
    toggleMute: () => sound.toggleMute(),
    toggleFullscreen: () => shell?.fullscreen(),
    play: () => void video?.play(),
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
      toggleFullscreen: controls.toggleFullscreen,
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
  const key = (k: Parameters<typeof bindingOf>[0], label: string) => (bindingOf(k) ? `${label} (${bindingOf(k)})` : label);
</script>

<Shell bind:this={shell} {controls} {menu} pinned={!engine?.playing} onvolume={(s) => sound.step(s)}>
  <video bind:this={video} playsinline {ontimeupdate} ondurationchange={ontimeupdate}></video>
  {#if failure}
    <div class="center">
      {failure === 'subs'
        ? 'This video is for subscribers only.'
        : failure === 'gone'
          ? 'This video is gone.'
          : failure === 'geo'
            ? 'Not available in your country.'
            : 'The video didn’t load.'}
    </div>
  {:else if engine?.waiting}
    <div class="spinner" aria-label="Loading"></div>
  {/if}
  {#if stats && engine}<Stats read={() => engine!.stats()} onclose={() => (stats = false)} />{/if}

  {#snippet top()}
    <div class="title">
      <b>{info?.title || 'Video'}</b>
      {#if info}<span>{[info.game, formatDate(new Date(info.publishedAt))].filter(Boolean).join(' · ')}</span>{/if}
    </div>
    {#if onclose}
      <button class="ctl" aria-label="Close" use:tooltip={'Close the video'} onclick={onclose}><Icon name="x" size={16} /></button>
    {/if}
  {/snippet}

  {#snippet bar()}
    <div class="rows">
      <SeekBar {duration} {time} {buffered} storyboard={board} onwantpreview={() => void playerIpc.storyboard(id).then((b) => (board = b)).catch(() => {})} onseek={seek} />
      <div class="row">
        <button class="ctl" aria-label={engine?.playing ? 'Pause' : 'Play'} use:tooltip={key('player.playPause', engine?.playing ? 'Pause' : 'Play')} onclick={controls.togglePlay}>
          <Icon name={engine?.playing ? 'pause' : 'play'} size={18} />
        </button>
        <Volume volume={sound.volume} muted={sound.muted} boost={sound.boost} onchange={sound.set} onboost={sound.setBoost} />
        <span class="clock">{clock(time)} / {clock(duration)}</span>
        <span class="grow"></span>
        {#if engine}
          <SettingsMenu
            levels={engine.levels}
            level={engine.level}
            choice={engine.choice}
            onchoose={(c) => void engine!.choose(c)}
            {stats}
            onstats={(on) => (stats = on)}
            {speed}
            onspeed={(s) => (speed = s)}
          />
        {/if}
        <button class="ctl" aria-label="Fullscreen" use:tooltip={key('player.fullscreen', 'Fullscreen')} onclick={controls.toggleFullscreen}>
          <Icon name="expand" size={16} />
        </button>
      </div>
    </div>
  {/snippet}
</Shell>

<style>
  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 16px;
    text-align: center;
    color: #ddd;
    background: rgba(0, 0, 0, 0.55);
  }
  .spinner {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 36px;
    height: 36px;
    margin: -18px 0 0 -18px;
    border: 3px solid rgba(255, 255, 255, 0.2);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
    pointer-events: none;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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
