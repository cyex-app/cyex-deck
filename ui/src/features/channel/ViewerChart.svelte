<!-- V106-18 Viewers over time for a live tab channel: one point a minute from the live
     check, memory only. Mounted by a popover while it's open, so it costs nothing closed. -->
<script lang="ts">
  import { tabsIpc } from '../../ipc/tabs';
  import type { ViewerHistory } from '../../ipc/bindings/ViewerHistory';
  import { formatDuration, formatNumber, formatTime } from '../../lib/format';
  import { graph, nearest, peakOf, pointAt } from '../../lib/sparkline';

  interface Props {
    channelId: string;
  }

  let { channelId }: Props = $props();

  const W = 288;
  const H = 96;

  let history = $state<ViewerHistory | null>(null);
  let sel = $state<number | null>(null);
  let now = $state(Date.now());

  async function load() {
    try {
      history = await tabsIpc.viewers(channelId);
    } catch {
      history = null;
    }
    now = Date.now();
  }

  $effect(() => {
    void load();
    const off = tabsIpc.onLive(() => void load());
    return () => void off.then((f) => f());
  });

  const series = $derived((history?.points ?? []) as [number, number][]);
  const from = $derived(history ? Date.parse(history.startedAt) / 1000 : undefined);
  const g = $derived(graph(series, W, H, from));
  const peak = $derived(peakOf(series) ?? history?.peak ?? null);
  const last = $derived(series.length ? series[series.length - 1] : null);
  /** The "before" band's share of the width, when it's wide enough for its label (40 px). */
  const before = $derived(g.x0 > 0 ? g.x0 : 0);
  const marker = $derived(sel === null || !series[sel] ? null : pointAt(series, sel, W, H, from));

  const clock = (unix: number) => formatTime(new Date(unix * 1000));
  const since = $derived.by(() => {
    if (!history) return '';
    const mins = Math.max(0, Math.floor((now - Date.parse(history.startedAt)) / 60_000));
    return `Live since ${formatTime(new Date(history.startedAt))} · ${formatDuration(Math.max(60, mins * 60))}`;
  });

  function move(e: PointerEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (!r.width) return;
    const i = nearest(series, ((e.clientX - r.left) / r.width) * W, W, from);
    sel = i < 0 ? null : i;
  }

  function key(e: KeyboardEvent) {
    if (!series.length) return;
    const at = sel ?? series.length - 1;
    const to =
      e.key === 'ArrowLeft' ? at - 1 : e.key === 'ArrowRight' ? at + 1 : e.key === 'Home' ? 0 : e.key === 'End' ? series.length - 1 : null;
    if (to === null) return;
    e.preventDefault();
    sel = Math.min(series.length - 1, Math.max(0, to));
  }

  const readout = $derived(sel !== null && series[sel] ? `${clock(series[sel][0])} · ${formatNumber(series[sel][1])} viewers` : '');
  const pct = (n: number, total: number) => (n / total) * 100;
</script>

<div class="chart">
  {#if history && last}
    <div class="head">
      <span>Now <b>{formatNumber(last[1])}</b></span>
      <span>{#if peak}Peak <b>{formatNumber(peak[1])}</b> at {clock(peak[0])}{/if}</span>
    </div>
    <p class="since">{since}</p>
  {/if}
  {#if g.line}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex -->
    <div
      class="plot"
      role="group"
      tabindex="0"
      aria-label="Viewers over time"
      onpointermove={move}
      onpointerleave={() => (sel = null)}
      onkeydown={key}
      onblur={() => (sel = null)}
    >
      <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true">
        {#if before > 0}<rect class="before" x="0" y="0" width={before} height={H} />{/if}
        <path class="area" d={g.area} />
        <polyline class="line" points={g.line} />
      </svg>
      {#if before >= 40}
        <span class="band" style:width="{pct(before, W)}%">Not tracked</span>
      {/if}
      {#if marker}
        <span class="hair" style:left="{pct(marker[0], W)}%"></span>
        <span class="dot" style:left="{pct(marker[0], W)}%" style:top="{pct(marker[1], H)}%"></span>
        <span class="tip" style:left="{Math.min(82, Math.max(18, pct(marker[0], W)))}%">{readout}</span>
      {/if}
    </div>
    <span class="sr-only" aria-live="polite">{readout}</span>
  {:else}
    <p class="empty">The chart fills in once a minute.</p>
  {/if}
</div>

<style>
  .chart {
    width: 312px;
    max-width: 100%;
    padding: 10px 12px 12px;
    font-size: 0.82rem;
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: var(--text);
  }
  b {
    font-weight: 700;
  }
  .since,
  .empty {
    margin: 2px 0 8px;
    color: var(--muted);
    font-size: 0.75rem;
  }
  .plot {
    position: relative;
    width: 100%;
    max-width: 288px;
    height: 96px;
    margin-top: 18px;
    border-radius: var(--radius);
    outline-offset: 2px;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  .before {
    fill: var(--surface-hover);
  }
  .area {
    fill: var(--accent-dim);
  }
  .line {
    fill: none;
    stroke: var(--accent-text);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
    stroke-linejoin: round;
  }
  .band {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 4px;
    box-sizing: border-box;
    color: var(--muted);
    font-size: 0.72rem;
    text-align: center;
    pointer-events: none;
  }
  .hair {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--border-strong, var(--muted));
    pointer-events: none;
  }
  .dot {
    position: absolute;
    width: 8px;
    height: 8px;
    margin: -4px 0 0 -4px;
    border-radius: 50%;
    background: var(--accent-text);
    box-shadow: 0 0 0 2px var(--bg);
    pointer-events: none;
  }
  .tip {
    position: absolute;
    top: -20px;
    transform: translateX(-50%);
    color: var(--text);
    font-size: 0.75rem;
    white-space: nowrap;
    pointer-events: none;
  }
</style>
