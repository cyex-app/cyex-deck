<!-- P3 Video stats over the picture, refreshed every second. -->
<script lang="ts">
  import { formatNumber } from '../../lib/format';
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
  <button class="close" aria-label="Close" onclick={onclose}>×</button>
  <span>Resolution</span><b>{s.resolution}</b>
  <span>Quality</span><b>{s.level}</b>
  <span>Bitrate</span><b>{kbps(s.bitrate)}</b>
  <span>Bandwidth</span><b>{kbps(s.bandwidth)}</b>
  {#if s.latency !== null}<span>Latency</span><b>{s.latency.toFixed(1)} s</b>{/if}
  <span>Buffer</span><b>{s.buffer.toFixed(1)} s</b>
  <span>Dropped frames</span><b>{s.dropped}</b>
</div>

<style>
  .stats {
    position: absolute;
    top: 10px;
    left: 10px;
    z-index: 3;
    display: grid;
    grid-template-columns: auto auto;
    gap: 2px 14px;
    padding: 8px 28px 8px 10px;
    background: rgba(0, 0, 0, 0.72);
    border-radius: var(--radius);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: #ddd;
  }
  b {
    font-weight: 600;
    color: #fff;
  }
  .close {
    position: absolute;
    top: 2px;
    right: 4px;
    padding: 0 4px;
    background: none;
    border: 0;
    color: #bbb;
    font-size: 1rem;
  }
</style>
