<!-- A clip (1.19, 1.20): a plain MP4 from Twitch's CDN, in the quality that fits the
     player unless one is picked (V101-17). A clip that was just made (P9) may need a few
     seconds before it plays. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { ClipPlayback } from '../../ipc/bindings/ClipPlayback';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { playerIpc } from '../../ipc/player';
  import { bindingOf } from '../../lib/hotkeys.svelte';
  import { playerMenu } from './menu';
  import SeekBar from './SeekBar.svelte';
  import Shell from './Shell.svelte';
  import { useSound } from './sound.svelte';
  import { clock } from './time';
  import { copyText, openLink, playerKeys } from './tools';
  import Volume from './Volume.svelte';

  interface Props {
    slug: string;
    tile: Tile | undefined;
    /** Once its details are in, for the frame around it. */
    onloaded?: (clip: ClipPlayback) => void;
    /** V102-08 Made just now: Twitch needs a few seconds before it plays. */
    fresh?: boolean;
    /** V103-14 Over the stream (1.20): fullscreen is the clip alone. */
    alone?: boolean;
  }

  let { slug, tile, onloaded, fresh = false, alone = false }: Props = $props();

  let video = $state<HTMLVideoElement>();
  let shell = $state<Shell>();
  let clip = $state<ClipPlayback | null>(null);
  let problem = $state<string | null>(null);
  let playing = $state(false);
  let time = $state(0);
  let duration = $state(0);
  let buffered = $state(0);
  let height = $state(720);
  /** A file picked from the menu, by its place in the list (landscape, then portrait, P11);
   * `null` fits the player with a landscape file. */
  let picked = $state<number | null>(null);
  let waitTimer: ReturnType<typeof setTimeout> | undefined;
  /** V102-08 The file failed and is being asked for again. */
  let retrying = $state(false);
  let fileTries = 0;

  // svelte-ignore state_referenced_locally
  const sound = useSound(() => tile);

  // svelte-ignore state_referenced_locally
  void load(slug, 0);
  onDestroy(() => clearTimeout(waitTimer));

  /** A fresh clip isn't there right away: ask again for half a minute. */
  async function load(s: string, tries: number) {
    try {
      const c = await playerIpc.clip(s);
      if (c) {
        clip = c;
        onloaded?.(c);
      } else if (tries < 10) {
        problem = 'Getting the clip ready…';
        waitTimer = setTimeout(() => void load(s, tries + 1), 3000);
      } else {
        problem = 'This clip is gone.';
      }
    } catch (e) {
      // V102-08 A clip just made can answer oddly until Twitch is done with it.
      if (fresh && tries < 10) {
        problem = 'Getting the clip ready…';
        waitTimer = setTimeout(() => void load(s, tries + 1), 3000);
      } else {
        problem = String(e);
      }
    }
  }

  /** V102-08 Twitch lists a new clip's files a moment before its servers have them, and a
   * video that failed doesn't ask again on its own. */
  function onerror() {
    if (!video || fileTries >= 10) {
      retrying = false;
      return;
    }
    fileTries++;
    retrying = true;
    clearTimeout(waitTimer);
    waitTimer = setTimeout(() => video?.load(), 2000);
  }

  /** The picked file, else the smallest one at least as tall as the player. */
  const all = $derived([...(clip?.qualities ?? []), ...(clip?.portrait ?? [])]);
  const file = $derived.by(() => {
    const q = clip?.qualities ?? [];
    if (picked !== null && all[picked]) return all[picked];
    const fits = q.filter((x) => x.height >= height);
    return fits.length ? fits[fits.length - 1] : q[0];
  });
  const src = $derived(file?.url);
  const name = (q: { height: number; frameRate: number }) => `${q.height}p${q.frameRate > 31 ? Math.round(q.frameRate) : ''}`;

  $effect(() => {
    if (video) sound.apply(video);
  });
  $effect(() => {
    const v = video;
    return () => v && sound.release(v);
  });

  $effect(() => {
    if (!video) return;
    // V103-03 Out of sight (0 px) keeps the file: another one would load again on return.
    const ro = new ResizeObserver(([e]) => {
      if (e.contentRect.height > 0) height = e.contentRect.height * devicePixelRatio;
    });
    ro.observe(video);
    return () => ro.disconnect();
  });

  function ontimeupdate() {
    if (!video) return;
    time = video.currentTime;
    duration = Number.isFinite(video.duration) ? video.duration : 0;
    const b = video.buffered;
    buffered = b.length ? b.end(b.length - 1) : 0;
  }

  const controls = {
    togglePlay: () => video && (video.paused ? void video.play() : video.pause()),
    toggleMute: () => sound.toggleMute(),
    toggleFullscreen: () => shell?.fullscreen(),
    play: () => void video?.play(),
    pause: () => video?.pause(),
    seekBy: (s: number) => video && (video.currentTime = Math.min(Math.max(0, video.currentTime + s), duration || Infinity)),
    media: () =>
      clip && { title: clip.clip.title || 'Clip', artist: clip.clip.channelName, artwork: clip.clip.thumbnail },
  };

  /** V101-17 */
  function menu() {
    const landscape = clip?.qualities.length ?? 0;
    return playerMenu({
      kind: 'clip',
      playing,
      muted: sound.muted,
      fullscreen: Boolean(document.fullscreenElement),
      keys: playerKeys(),
      quality: [
        { label: 'Auto', hint: picked === null && file ? name(file) : undefined, on: picked === null, pick: () => (picked = null) },
        ...all.map((q, i) => ({
          label: i < landscape ? name(q) : `Portrait ${name(q)}`,
          on: picked === i,
          pick: () => (picked = i),
        })),
      ],
      togglePlay: controls.togglePlay,
      toggleMute: controls.toggleMute,
      toggleFullscreen: controls.toggleFullscreen,
      link: `https://clips.twitch.tv/${slug}`,
      copy: copyText,
      open: openLink,
    });
  }
  const key = (k: Parameters<typeof bindingOf>[0], label: string) => (bindingOf(k) ? `${label} (${bindingOf(k)})` : label);
</script>

<Shell bind:this={shell} {controls} {menu} {alone} pinned={!playing} onvolume={(s) => sound.step(s)}>
  {#if src}
    <!-- The clip's place in the stream keeps going when the quality changes. Twitch clips
         have no captions. -->
    <!-- svelte-ignore a11y_media_has_caption -->
    <video
      bind:this={video}
      {src}
      autoplay
      playsinline
      onplay={() => (playing = true)}
      onpause={() => (playing = false)}
      onloadedmetadata={() => video && time && (video.currentTime = time)}
      oncanplay={() => (retrying = false)}
      {onerror}
      {ontimeupdate}
    ></video>
    {#if retrying}<div class="center over">Getting the clip ready…</div>{/if}
  {:else}
    <div class="center">{problem ?? ''}</div>
  {/if}

  {#snippet bar()}
    <div class="rows">
      <SeekBar {duration} {time} {buffered} onseek={(t) => video && (video.currentTime = t)} />
      <div class="row">
        <button class="ctl" aria-label={playing ? 'Pause' : 'Play'} use:tooltip={key('player.playPause', playing ? 'Pause' : 'Play')} onclick={controls.togglePlay}>
          <Icon name={playing ? 'pause' : 'play'} size={18} />
        </button>
        <Volume volume={sound.volume} muted={sound.muted} onchange={sound.set} />
        <span class="clock">{clock(time)} / {clock(duration)}</span>
        <span class="grow"></span>
        <button class="ctl" aria-label="Fullscreen" use:tooltip={key('player.fullscreen', 'Fullscreen')} onclick={controls.toggleFullscreen}>
          <Icon name="expand" size={16} />
        </button>
      </div>
    </div>
  {/snippet}
</Shell>

<style>
  .center.over {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .center {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 16px;
    color: var(--on-video-dim);
    text-align: center;
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
