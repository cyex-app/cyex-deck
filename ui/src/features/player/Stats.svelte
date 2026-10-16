<!-- P3 Video stats over the picture, refreshed every second. C8 Anchored to the drawn picture
     (not the bars around it), 12 px in. -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import { formatDecimal, formatNumber } from '../../lib/format';
  import type { Stats } from './engine.svelte';
  import { containRect } from './videoRect';

  let { read, video, onclose }: { read: () => Stats; video: HTMLVideoElement | undefined; onclose: () => void } = $props();

  let box = $state<HTMLElement>();
  let at = $state({ left: 0, top: 0 });

  // Follows the picture as the window, the stream's size or the layout changes.
  $effect(() => {
    const v = video;
    const parent = box?.offsetParent;
    if (!v || !parent) return;
    const place = () => {
      const pr = parent.getBoundingClientRect();
      const vr = v.getBoundingClientRect();
      const r = containRect(vr.width, vr.height, v.videoWidth, v.videoHeight);
      at = { left: vr.left - pr.left + r.left, top: vr.top - pr.top + r.top };
    };
    place();
    const ro = new ResizeObserver(place);
    ro.observe(v);
    ro.observe(parent);
    v.addEventListener('resize', place);
    v.addEventListener('loadedmetadata', place);
    return () => {
      ro.disconnect();
      v.removeEventListener('resize', place);
      v.removeEventListener('loadedmetadata', place);
    };
  });

  // svelte-ignore state_referenced_locally
  let s = $state(read());
  $effect(() => {
    const timer = setInterval(() => (s = read()), 1000);
    return () => clearInterval(timer);
  });

  const kbps = (bits: number) => `${formatNumber(Math.round(bits / 1000))} kbps`;
</script>

<div class="stats" role="status" bind:this={box} style:left="{at.left + 12}px" style:top="{at.top + 12}px">
  <div class="head">
    <b>Video stats</b>
    <IconButton icon="x" label="Close" size={13} onclick={onclose} />
  </div>
  <div class="grid">
    <span>Resolution</span><b>{s.resolution}</b>
    <span>Quality</span><b>{s.level}</b>
    <span>Bitrate</span><b>{kbps(s.bitrate)}</b>
    <span>Bandwidth</span><b>{kbps(s.bandwidth)}</b>
    {#if s.latency !== null}<span>Latency</span><b>{formatDecimal(s.latency)}s</b>{/if}
    <span>Buffer</span><b>{formatDecimal(s.buffer)}s</b>
    <span>Dropped frames</span><b>{s.dropped}</b>
  </div>
</div>

<style>
  .stats {
    position: absolute;
    top: 12px;
    left: 12px;
    max-width: calc(100% - 24px);
    z-index: var(--z-panel);
    padding: 4px 10px 8px;
    background: var(--scrim-strong);
    border-radius: var(--radius);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--on-video-dim);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .stats :global(.icon-btn) {
    width: 24px;
    height: 24px;
    margin-right: -6px;
    color: var(--on-video-dim);
  }
  .stats :global(.icon-btn:hover:not(:disabled)) {
    background: var(--scrim);
    color: var(--on-video);
  }
  .grid {
    display: grid;
    grid-template-columns: auto auto;
    gap: 2px 14px;
  }
  b {
    font-weight: 600;
    color: var(--on-video);
  }
</style>
