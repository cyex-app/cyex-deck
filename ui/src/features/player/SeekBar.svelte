<!-- The timeline of a VOD or clip. Hovering shows the time and, for VODs, a picture of
     that moment from Twitch's storyboard (P6). Drag or click to seek. -->
<script lang="ts">
  import type { Storyboard } from '../../ipc/bindings/Storyboard';
  import { clock } from './time';

  interface Props {
    duration: number;
    time: number;
    buffered: number;
    storyboard?: Storyboard | null;
    /** Asked for on the first hover. */
    onwantpreview?: () => void;
    onseek: (t: number) => void;
  }

  let { duration, time, buffered, storyboard, onwantpreview, onseek }: Props = $props();

  let track = $state<HTMLElement>();
  let hover = $state<number | null>(null);
  let dragging = $state(false);
  let asked = false;

  const shown = $derived(dragging && hover !== null ? hover : time);
  const pct = (t: number) => `${duration > 0 ? Math.min(100, (t / duration) * 100) : 0}%`;

  /** The storyboard picture for `t`: which sheet, and where on it. */
  const preview = $derived.by(() => {
    if (hover === null || !storyboard || storyboard.interval <= 0) return null;
    const b = storyboard;
    const per = b.cols * b.rows;
    const n = Math.min(Math.floor(hover / b.interval), Math.max(0, b.count - 1));
    const sheet = b.images[Math.floor(n / per)];
    if (!sheet) return null;
    const k = n % per;
    return { url: sheet, x: (k % b.cols) * b.width, y: Math.floor(k / b.cols) * b.height, w: b.width, h: b.height };
  });

  function at(e: PointerEvent): number {
    const r = track!.getBoundingClientRect();
    return Math.min(Math.max((e.clientX - r.left) / r.width, 0), 1) * duration;
  }

  function onmove(e: PointerEvent) {
    hover = at(e);
    if (!asked) {
      asked = true;
      onwantpreview?.();
    }
  }

  function ondown(e: PointerEvent) {
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    hover = at(e);
  }

  function onup(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    onseek(at(e));
  }
</script>

<div
  class="seek"
  bind:this={track}
  role="slider"
  tabindex="-1"
  aria-label="Seek"
  aria-valuemin={0}
  aria-valuemax={Math.round(duration)}
  aria-valuenow={Math.round(time)}
  onpointermove={onmove}
  onpointerleave={() => !dragging && (hover = null)}
  onpointerdown={ondown}
  onpointerup={onup}
>
  <div class="rail">
    <div class="buffered" style:width={pct(buffered)}></div>
    <div class="played" style:width={pct(shown)}></div>
  </div>
  {#if hover !== null}
    <div class="tip" style:left={pct(hover)}>
      {#if preview}
        <div
          class="thumb"
          style:width="{preview.w}px"
          style:height="{preview.h}px"
          style:background-image="url({preview.url})"
          style:background-position="-{preview.x}px -{preview.y}px"
        ></div>
      {/if}
      <span>{clock(hover)}</span>
    </div>
  {/if}
</div>

<style>
  .seek {
    position: relative;
    flex: 1;
    height: 18px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
  }
  .rail {
    position: relative;
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.25);
    border-radius: 2px;
    transition: height 0.1s;
  }
  .seek:hover .rail {
    height: 6px;
  }
  .buffered,
  .played {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 2px;
  }
  .buffered {
    background: rgba(255, 255, 255, 0.35);
  }
  .played {
    background: var(--accent-hover);
  }
  .tip {
    position: absolute;
    bottom: 22px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    pointer-events: none;
  }
  .tip span {
    padding: 1px 6px;
    background: rgba(0, 0, 0, 0.8);
    border-radius: var(--radius);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .thumb {
    border: 1px solid rgba(255, 255, 255, 0.5);
    border-radius: var(--radius);
    background-repeat: no-repeat;
  }
</style>
