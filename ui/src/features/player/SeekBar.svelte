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
    /** V1061-03 The hover tip's text (default: the clock). */
    format?: (t: number) => string;
    /** Neutral: the played part isn't the accent (the live rewind bar). */
    tone?: 'accent' | 'neutral';
    valueText?: string;
  }

  let { duration, time, buffered, storyboard, onwantpreview, onseek, format = clock, tone = 'accent', valueText }: Props = $props();

  let track = $state<HTMLElement>();
  let hover = $state<number | null>(null);
  let dragging = $state(false);
  let asked = false;

  const shown = $derived(dragging && hover !== null ? hover : time);
  const frac = (t: number) => (duration > 0 ? Math.min(1, Math.max(0, t / duration)) : 0);
  const pct = (t: number) => `${frac(t) * 100}%`;

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
  aria-valuenow={Math.round(shown)}
  aria-valuetext={valueText}
  onpointermove={onmove}
  onpointerleave={() => !dragging && (hover = null)}
  onpointerdown={ondown}
  onpointerup={onup}
>
  <div class="rail">
    <!-- s1-13 Scaled, not sized: no layout on every time update. -->
    <div class="buffered" style:transform="scaleX({frac(buffered)})"></div>
    <div class="played" class:neutral={tone === 'neutral'} style:transform="scaleX({frac(shown)})"></div>
  </div>
  <!-- d-8 The thumb, while hovering or dragging. -->
  <div class="knob-track" style:transform="translateX({pct(shown)})" aria-hidden="true">
    <div class="knob" class:on={hover !== null || dragging} class:neutral={tone === 'neutral'}></div>
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
      <span>{format(hover)}</span>
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
    background: var(--on-video-faint);
    border-radius: var(--radius-sm);
    transition: transform var(--fast) var(--ease);
  }
  .seek:hover .rail {
    transform: scaleY(1.5);
  }
  .buffered,
  .played {
    position: absolute;
    inset: 0;
    border-radius: var(--radius-sm);
    transform-origin: left center;
  }
  .knob-track {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .knob {
    position: absolute;
    top: 50%;
    left: -6px;
    width: 12px;
    height: 12px;
    margin-top: -6px;
    border-radius: 50%;
    background: var(--accent-hover);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--on-video) 45%, transparent);
    opacity: 0;
    transform: scale(0.5);
    transition:
      opacity var(--fast) var(--ease),
      transform var(--fast) var(--ease);
  }
  .knob.neutral {
    background: var(--on-video);
  }
  .knob.on {
    opacity: 1;
    transform: none;
  }
  .buffered {
    background: color-mix(in srgb, var(--on-video) 35%, transparent);
  }
  .played {
    background: var(--accent-hover);
  }
  .played.neutral {
    background: color-mix(in srgb, var(--on-video) 80%, transparent);
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
    background: var(--scrim-strong);
    border-radius: var(--radius);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .thumb {
    border: 1px solid color-mix(in srgb, var(--on-video) 50%, transparent);
    border-radius: var(--radius);
    background-repeat: no-repeat;
  }
</style>
