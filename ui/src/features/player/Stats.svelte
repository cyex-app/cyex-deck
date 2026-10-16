<!-- P3 Video stats over the picture, refreshed every second. -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import { formatDecimal, formatNumber } from '../../lib/format';
  import type { Stats } from './engine.svelte';

  let { read, onclose }: { read: () => Stats; onclose: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let s = $state(read());
  $effect(() => {
    const timer = setInterval(() => (s = read()), 1000);
    return () => clearInterval(timer);
  });

  const kbps = (bits: number) => `${formatNumber(Math.round(bits / 1000))} kbps`;
</script>

<div class="stats" role="status">
  <div class="head">
    <b>Video stats</b>
    <IconButton icon="x" label="Close" size={13} onclick={onclose} />
  </div>
  <div class="grid">
    <span>Resolution</span><b>{s.resolution}</b>
    <span>Quality</span><b>{s.level}</b>
    <span>Bitrate</span><b>{kbps(s.bitrate)}</b>
    <span>Bandwidth</span><b>{kbps(s.bandwidth)}</b>
    {#if s.latency !== null}<span>Latency</span><b>{formatDecimal(s.latency)} s</b>{/if}
    <span>Buffer</span><b>{formatDecimal(s.buffer)} s</b>
    <span>Dropped frames</span><b>{s.dropped}</b>
  </div>
</div>

<style>
  .stats {
    position: absolute;
    top: 12px;
    left: 12px;
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
