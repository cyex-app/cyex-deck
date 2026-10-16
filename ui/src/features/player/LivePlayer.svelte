<!-- A live stream (1.1) with the ad filter (1.2) behind it in Rust. Plays with the tile's
     account when it has a GQL sign-in (D19). Volume is kept per tile. While the channel's
     pages show it plays small in a corner, dragged to any corner (P4, V101-25). -->
<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { AdState } from '../../ipc/bindings/AdState';
  import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
  import type { Corner } from '../../ipc/bindings/Corner';
  import type { PlayerSession } from '../../ipc/bindings/PlayerSession';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { playerIpc } from '../../ipc/player';
  import { bindingOf } from '../../lib/hotkeys.svelte';
  import { settings } from '../../lib/settings.svelte';
  import Celebration from './Celebration.svelte';
  import { Engine } from './engine.svelte';
  import { playerMenu } from './menu';
  import { type Choice, qualityOptions } from './quality';
  import SettingsMenu from './SettingsMenu.svelte';
  import Shell from './Shell.svelte';
  import Stats from './Stats.svelte';
  import { useSound } from './sound.svelte';
  import { copyText, openLink, pipFor, playerKeys } from './tools';
  import Volume from './Volume.svelte';

  interface Props {
    tile: Tile;
    info: ChannelInfo | undefined;
    /** 1.20 A clip or story plays over it: silent meanwhile. */
    hushed?: boolean;
    /** P4 Small in a corner while the channel's pages show. */
    mini?: boolean;
    onexpand?: () => void;
    onclose?: () => void;
    /** 1.58 Pops the chat out, when the tile shows both. */
    onpopchat?: () => void;
    /** P9 */
    onclip?: () => void;
    /** V102-08 Why clipping is off here, e.g. an anonymous tab: the button shows greyed. */
    clipOff?: string | null;
    /** V103-07 Another player plays this stream: this one starts paused (and its tile muted). */
    held?: boolean;
  }

  let { tile, info, hushed = false, mini = false, onexpand, onclose, onpopchat, onclip, clipOff = null, held = false }: Props = $props();

  let video = $state<HTMLVideoElement>();
  let shell = $state<Shell>();
  let engine = $state<Engine>();
  let session: PlayerSession | null = null;
  let ad = $state<AdState>('none');
  let stats = $state(false);
  let tries = 0;
  let retry: ReturnType<typeof setTimeout> | undefined;

  // svelte-ignore state_referenced_locally
  const sound = useSound(() => tile, () => hushed, true);

  const stopAds = playerIpc.onAds((id, state) => {
    if (id === session?.id) ad = state;
  });

  // Rust hands out a new tile object on every tab edit (volume, sizes): only these
  // values restart the stream.
  const login = $derived(tile.channel.login);
  const account = $derived(tile.account);
  const lowLatency = $derived(settings.v.player.lowLatency);

  $effect(() => {
    if (!video) return;
    const e = new Engine(video, true, lowLatency);
    engine = e;
    // The setting is where new players start; a change doesn't restart this one.
    void start(e, untrack(() => settings.v.player.quality), login, account, untrack(() => held));
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

  // Network trouble: a fresh session, three times, then a button.
  $effect(() => {
    if (engine?.failure !== 'network' || tries >= 3) return;
    retry = setTimeout(() => engine && void start(engine, engine.choice, login, account), 3000 * ++tries);
  });

  onDestroy(() => {
    clearTimeout(retry);
    void stopAds.then((f) => f());
    if (session) void playerIpc.close(session.id);
  });

  async function start(e: Engine, choice: Choice, channel: string, as: string | null, hold = false) {
    if (session) void playerIpc.close(session.id);
    session = await playerIpc.live(channel, as);
    ad = 'none';
    await e.start(session, choice, -1, hold);
  }

  function reload() {
    tries = 0;
    if (engine) void start(engine, engine.choice, login, account);
  }

  const controls = {
    togglePlay: () => video && (video.paused ? void video.play() : video.pause()),
    toggleMute: () => sound.toggleMute(),
    // P4 Small, a double-click brings the stream back instead.
    toggleFullscreen: () => (mini ? onexpand?.() : shell?.fullscreen()),
    play: () => void video?.play(),
    pause: () => video?.pause(),
    media: () => ({ title: info?.stream?.title || tile.channel.name, artist: tile.channel.name, artwork: info?.avatar }),
  };

  /** V101-17 */
  function menu() {
    const e = engine;
    return playerMenu({
      kind: 'live',
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
      pip: pipFor(video),
      stats: { on: stats, toggle: () => (stats = !stats) },
      reload,
      link: `https://www.twitch.tv/${login}`,
      copy: copyText,
      open: openLink,
    });
  }

  // V101-25 The small stream follows the pointer and lands in the nearest corner, which
  // new ones start in too.
  let box = $state<HTMLElement>();
  /** Picked here, until the setting has it. */
  let placed = $state<Corner | null>(null);
  const corner = $derived(placed ?? settings.v.player.miniCorner ?? 'bottomRight');
  let drag = $state<{ x: number; y: number; dx: number; dy: number; moved: boolean } | null>(null);
  /** The click that ends a drag doesn't pause the stream. */
  let dropped = false;

  function ondown(e: PointerEvent) {
    dropped = false;
    if (!mini || !box || e.button !== 0 || (e.target as HTMLElement).closest('button')) return;
    drag = { x: e.clientX, y: e.clientY, dx: 0, dy: 0, moved: false };
    box.setPointerCapture(e.pointerId);
  }

  function onmove(e: PointerEvent) {
    if (!drag || !box?.parentElement) return;
    let dx = e.clientX - drag.x;
    let dy = e.clientY - drag.y;
    if (!drag.moved && Math.hypot(dx, dy) < 5) return;
    // Kept inside the stream side.
    const area = box.parentElement.getBoundingClientRect();
    const r = box.getBoundingClientRect();
    const left = r.left - drag.dx;
    const top = r.top - drag.dy;
    dx = Math.min(Math.max(dx, area.left - left), area.right - (left + r.width));
    dy = Math.min(Math.max(dy, area.top - top), area.bottom - (top + r.height));
    drag = { ...drag, dx, dy, moved: true };
  }

  async function onup() {
    const d = drag;
    drag = null;
    if (!d?.moved || !box?.parentElement) return;
    dropped = true;
    const area = box.parentElement.getBoundingClientRect();
    const before = box.getBoundingClientRect();
    const x = before.left + before.width / 2 - area.left < area.width / 2 ? 'Left' : 'Right';
    const next = `${before.top + before.height / 2 - area.top < area.height / 2 ? 'top' : 'bottom'}${x}` as Corner;
    placed = next;
    await tick();
    const after = box.getBoundingClientRect();
    box.animate([{ translate: `${before.left - after.left}px ${before.top - after.top}px` }, { translate: '0 0' }], {
      duration: 180,
      easing: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
    });
    if (next !== settings.v.player.miniCorner) await settings.patch('player', { miniCorner: next });
    placed = null;
  }

  function onclickcapture(e: MouseEvent) {
    if (!dropped) return;
    dropped = false;
    e.stopPropagation();
  }

  const failure = $derived(engine?.failure);
  const key = (id: Parameters<typeof bindingOf>[0], label: string) =>
    bindingOf(id) ? `${label} (${bindingOf(id)})` : label;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="live {mini ? corner : ''}"
  class:mini
  class:dragging={drag?.moved}
  bind:this={box}
  style:translate={drag?.moved ? `${drag.dx}px ${drag.dy}px` : null}
  onpointerdown={ondown}
  onpointermove={onmove}
  onpointerup={onup}
  onpointercancel={() => (drag = null)}
  {onclickcapture}
>
  <Shell bind:this={shell} {controls} {menu} pinned={!engine?.playing} onvolume={(s) => sound.step(s)}>
    <video bind:this={video} playsinline></video>
    {#if engine?.choice === 'audio'}
      <div class="center">
        <Avatar src={info?.avatar} name={tile.channel.name} size={72} />
        <span>Audio only</span>
      </div>
    {/if}
    {#if failure}
      <div class="center">
        <span>
          {failure === 'offline'
            ? `${tile.channel.name} went offline.`
            : failure === 'geo'
              ? 'Not available in your country.'
              : 'The stream didn’t load.'}
        </span>
        {#if failure !== 'offline'}
          <button class="retry" onclick={() => ((tries = 0), engine && void start(engine, engine.choice, login, account))}>
            <Icon name="refresh" size={14} />Try again
          </button>
        {/if}
      </div>
    {:else if ad === 'waiting'}
      <div class="center">
        <span>Ad break. The stream comes back right after.</span>
      </div>
    {:else if engine?.holding}
      <!-- V103-07 A click anywhere plays it (Shell). -->
      <div class="center held">Playing elsewhere. Click to play here.</div>
    {:else if engine?.waiting}
      <div class="spinner" aria-label="Loading"></div>
    {/if}
    {#if ad === 'backup' || ad === 'showing'}
      <span class="chip">{ad === 'backup' ? 'Skipping an ad break' : 'Ad'}</span>
    {/if}
    {#if !mini}<Celebration login={tile.channel.login} />{/if}
    {#if stats && engine}
      <Stats read={() => engine!.stats()} onclose={() => (stats = false)} />
    {/if}
    {#if mini}
      <div class="mini-bar">
        <button class="ctl" aria-label="Back to the stream" use:tooltip={'Back to the stream'} onclick={onexpand}>
          <Icon name="expand" size={14} />
        </button>
        <button class="ctl" aria-label="Close" use:tooltip={'Stop'} onclick={onclose}><Icon name="x" size={14} /></button>
      </div>
    {/if}

    {#snippet bar()}
      {#if !mini}
        <button class="ctl" aria-label={engine?.playing ? 'Pause' : 'Play'} use:tooltip={key('player.playPause', engine?.playing ? 'Pause' : 'Play')} onclick={controls.togglePlay}>
          <Icon name={engine?.playing ? 'pause' : 'play'} size={18} />
        </button>
        <Volume volume={sound.volume} muted={sound.muted} boost={sound.boost} onchange={sound.set} onboost={sound.setBoost} />
        <span class="live-badge">LIVE</span>
        <span class="grow"></span>
        {#if onclip}
          <button class="ctl" aria-label="Clip" use:tooltip={{ label: 'Clip', hint: 'Clips the last seconds of the stream.' }} onclick={onclip}>
            <Icon name="scissors" size={17} />
          </button>
        {:else if clipOff}
          <button class="ctl off" aria-label="Clip" aria-disabled="true" use:tooltip={{ label: 'Clip', hint: clipOff }}>
            <Icon name="scissors" size={17} />
          </button>
        {/if}
        {#if onpopchat}
          <button class="ctl" aria-label="Pop out chat" use:tooltip={{ label: 'Pop out chat', hint: 'Chat in its own window, the stream here.' }} onclick={onpopchat}>
            <Icon name="popout" size={16} />
          </button>
        {/if}
        {#if engine}
          <SettingsMenu
            levels={engine.levels}
            level={engine.level}
            choice={engine.choice}
            onchoose={(c) => void engine!.choose(c)}
            {stats}
            onstats={(on) => (stats = on)}
          />
        {/if}
        <button class="ctl" aria-label="Fullscreen" use:tooltip={key('player.fullscreen', 'Fullscreen')} onclick={controls.toggleFullscreen}>
          <Icon name="expand" size={16} />
        </button>
      {/if}
    {/snippet}
  </Shell>
</div>

<style>
  /* V102-08 */
  .ctl.off {
    opacity: 0.4;
    cursor: default;
  }
  .live {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  .mini {
    position: absolute;
    z-index: 4;
    width: min(320px, 45%);
    aspect-ratio: 16 / 9;
    flex: none;
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    touch-action: none;
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
  .dragging {
    cursor: grabbing;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.6);
  }
  .dragging :global(.shell) {
    cursor: grabbing;
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 16px;
    text-align: center;
    color: var(--on-video-dim);
    background: var(--scrim);
  }
  .center.held {
    font-size: 0.85rem;
    pointer-events: none;
  }
  .retry {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: color-mix(in srgb, var(--on-video) 14%, transparent);
    border: 0;
    border-radius: var(--radius);
    color: var(--on-video);
  }
  .spinner {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 36px;
    height: 36px;
    margin: -18px 0 0 -18px;
    border: 3px solid var(--on-video-faint);
    border-top-color: var(--on-video);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
    pointer-events: none;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .chip {
    position: absolute;
    top: 10px;
    right: 10px;
    z-index: 3;
    padding: 3px 8px;
    background: var(--scrim-strong);
    border-radius: var(--radius);
    font-size: 0.75rem;
    pointer-events: none;
  }
  .live-badge {
    margin-left: 6px;
    padding: 0 5px;
    background: var(--live);
    border-radius: var(--radius);
    font-size: 0.7rem;
    font-weight: 700;
  }
  .grow {
    flex: 1;
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
</style>
