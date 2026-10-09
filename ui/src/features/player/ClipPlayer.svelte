<!-- A clip (1.19, 1.20): a plain MP4 from Twitch's CDN, in the quality that fits the
     player. A clip that was just made (P9) may need a few seconds before it plays. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { ClipPlayback } from '../../ipc/bindings/ClipPlayback';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { playerIpc } from '../../ipc/player';
  import { bindingOf } from '../../lib/hotkeys.svelte';
  import SeekBar from './SeekBar.svelte';
  import Shell from './Shell.svelte';
  import { useSound } from './sound.svelte';
  import { clock } from './time';
  import Volume from './Volume.svelte';

  interface Props {
    slug: string;
    tile: Tile | undefined;
    /** Once its details are in, for the frame around it. */
    onloaded?: (clip: ClipPlayback) => void;
  }

  let { slug, tile, onloaded }: Props = $props();

  let video = $state<HTMLVideoElement>();
  let shell = $state<Shell>();
  let clip = $state<ClipPlayback | null>(null);
  let problem = $state<string | null>(null);
  let playing = $state(false);
  let time = $state(0);
  let duration = $state(0);
  let buffered = $state(0);
  let height = $state(720);
  let waitTimer: ReturnType<typeof setTimeout> | undefined;

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
      problem = String(e);
    }
  }

  /** The smallest file at least as tall as the player. */
  const src = $derived.by(() => {
    const q = clip?.qualities ?? [];
    const fits = q.filter((x) => x.height >= height);
    return (fits.length ? fits[fits.length - 1] : q[0])?.url;
  });

  $effect(() => {
    if (video) sound.apply(video);
  });

  $effect(() => {
    if (!video) return;
    const ro = new ResizeObserver(([e]) => (height = e.contentRect.height * devicePixelRatio));
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
  };
  const key = (k: Parameters<typeof bindingOf>[0], label: string) => (bindingOf(k) ? `${label} (${bindingOf(k)})` : label);
</script>

<Shell bind:this={shell} {controls} pinned={!playing} onvolume={(s) => sound.step(s)}>
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
      {ontimeupdate}
    ></video>
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
  .center {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 16px;
    color: #ddd;
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
