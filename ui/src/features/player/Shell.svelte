<!-- The frame every player shares: controls that show while the mouse moves, a press to
     play or pause, double-click for fullscreen (1.58, V101-27), scroll for volume (P2),
     right-click for the player's menu (V101-17). The player used or opened last gets the
     hotkeys (S2) and the media keys (V101-29). -->
<script lang="ts">
  import { onDestroy, type Snippet } from 'svelte';

  import { type MenuEntry, openMenu } from '../../components/menu.svelte';
  import { type PlayerControls, players, toggleFullscreen } from './players.svelte';
  import SeekReadout from './SeekReadout.svelte';
  import { hitInside } from './shell';

  interface Props {
    controls: PlayerControls;
    /** Controls stay up, e.g. while paused. */
    pinned?: boolean;
    /** P2 Scroll steps, + louder. */
    onvolume?: (step: number) => void;
    /** V101-17 The right-click menu. */
    menu?: () => MenuEntry[];
    /** V103-14 Fullscreen takes only this player, not the surface around it: a clip over
     *  the stream fills the screen instead of staying in its box. */
    alone?: boolean;
    top?: Snippet;
    bar?: Snippet;
    children: Snippet;
  }

  let { controls, pinned = false, onvolume, menu, alone = false, top, bar, children }: Props = $props();

  let root = $state<HTMLElement>();
  let readout = $state<SeekReadout>();
  let active = $state(false);
  let idle: ReturnType<typeof setTimeout> | undefined;
  /** d-7 The first press of a click toggled play. */
  let toggled = false;

  // svelte-ignore state_referenced_locally
  const self: PlayerControls = {
    togglePlay: () => controls.togglePlay(),
    toggleMute: () => controls.toggleMute(),
    toggleFullscreen: () => controls.toggleFullscreen(),
    play: () => controls.play?.(),
    pause: () => controls.pause?.(),
    seekBy: controls.seekBy && ((s) => controls.seekBy?.(s)),
    toLive: controls.toLive && (() => controls.toLive?.()),
    volumeStep: (d) => {
      onvolume?.(d);
      wake();
    },
    visible: () => !!root && root.checkVisibility() && !root.closest('.mini'),
    say: (t) => readout?.show(t),
    media: () => controls.media?.() ?? null,
  };
  const unregister = players.register(self);
  onDestroy(() => {
    unregister();
    clearTimeout(idle);
  });

  /** For the player's `toggleFullscreen`. The surface around the player goes along, so
   *  what shows over it stays on screen (V101-26). */
  export function fullscreen() {
    if (root) void toggleFullscreen((!alone && root.closest<HTMLElement>('[data-fullscreen]')) || root);
  }

  /** V1061-04 A short readout over the picture. */
  export function say(t: string) {
    readout?.show(t);
  }

  function wake() {
    active = true;
    clearTimeout(idle);
    idle = setTimeout(() => (active = false), 2500);
  }

  /** Whether the event landed on this player's own controls (V101-27). */
  const on = (e: Event, selector: string) => !root || hitInside(e.target as HTMLElement, root, selector);

  const PICTURE_SKIP = '.bar, .top, button, input, a';

  /** d-7 A press on the picture (not the controls) plays or pauses at once. The second press
   *  of a double click doesn't: the double click puts the play state back and goes fullscreen. */
  function onmousedown(e: MouseEvent) {
    if (e.button !== 0 || e.detail >= 2) return;
    toggled = !on(e, PICTURE_SKIP);
    if (toggled) controls.togglePlay();
  }

  function ondblclick(e: MouseEvent) {
    if (on(e, PICTURE_SKIP)) return;
    if (toggled) controls.togglePlay();
    toggled = false;
    self.toggleFullscreen();
  }

  function onwheel(e: WheelEvent) {
    if (!onvolume || on(e, '.menu')) return;
    e.preventDefault();
    onvolume(e.deltaY < 0 ? 1 : -1);
    wake();
  }

  function oncontextmenu(e: MouseEvent) {
    if (!menu || on(e, 'input')) return;
    e.preventDefault();
    e.stopPropagation();
    players.use(self);
    openMenu(e, menu());
  }
</script>

<!-- Presses on the picture mirror the play button and F; the controls are real buttons. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="shell"
  class:show={active || pinned}
  bind:this={root}
  onpointermove={wake}
  onpointerdown={() => players.use(self)}
  onpointerup={(e) => (e.target as HTMLElement).closest<HTMLElement>('.ctl')?.blur()}
  onpointerleave={() => (active = false)}
  {onmousedown}
  {ondblclick}
  {onwheel}
  {oncontextmenu}
>
  {@render children()}
  <SeekReadout bind:this={readout} />
  {#if top}<div class="top">{@render top()}</div>{/if}
  {#if bar}<div class="bar">{@render bar()}</div>{/if}
</div>

<style>
  .shell {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    background: #000;
    color: var(--on-video);
    /* Video stays dark in both themes, so focus rings are white in here. */
    --focus-ring: var(--on-video);
    overflow: hidden;
    cursor: none;
  }
  .shell.show {
    cursor: default;
  }
  .top,
  .bar {
    position: absolute;
    left: 0;
    right: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--base) var(--ease);
  }
  .show .top,
  .show .bar {
    opacity: 1;
  }
  .top {
    top: 0;
    padding: 10px 12px 26px;
    background: linear-gradient(var(--scrim-strong), transparent);
  }
  .bar {
    bottom: 0;
    padding: 26px 8px 6px;
    background: linear-gradient(transparent, var(--scrim-strong));
  }
  .shell:not(.show) .top :global(*),
  .shell:not(.show) .bar :global(*) {
    pointer-events: none;
  }
  .shell :global(video) {
    flex: 1;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    object-fit: contain;
    background: #000;
  }
  .shell :global(.ctl) {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--on-video);
    opacity: 0.9;
  }
  .shell :global(.ctl:hover) {
    opacity: 1;
    background: color-mix(in srgb, var(--on-video) 12%, transparent);
  }
  .shell :global(.ctl.on) {
    color: #9cbcff;
  }
  :global(.shell:fullscreen) {
    border: 0;
  }
</style>
