<!-- V106-18 Viewers over time for a live tab channel: one point a minute from the live
     check, memory only. Mounted by a popover while it's open, so it costs nothing closed. -->
<script lang="ts">
  import { tabsIpc } from '../../ipc/tabs';
  import type { ViewerHistory } from '../../ipc/bindings/ViewerHistory';
  import { formatDuration, formatNumber, formatTime } from '../../lib/format';
  import { graph, inBand, nearest, peakOf, pointAt } from '../../lib/sparkline';

  interface Props {
    channelId: string;
  }

  let { channelId }: Props = $props();

  const W = 288;
  const H = 96;
  const uid = $props.id();

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
  // c-6 Padded: +4 % reads as a small rise, not a full-height ramp.
  const g = $derived(graph(series, W, H, from, true));
  const peak = $derived(peakOf(series) ?? history?.peak ?? null);
  const last = $derived(series.length ? series[series.length - 1] : null);
  /** Width of the grey "before" band (time before the app watched), in viewBox units. */
  const before = $derived(g.x0 > 0 ? g.x0 : 0);
  /** `sel` of -1 is the band: it reads "Not tracked" and has no marker. */
  const marker = $derived(sel === null || sel < 0 || !series[sel] ? null : pointAt(series, sel, W, H, from, true));

  const clock = (unix: number) => formatTime(new Date(unix * 1000));
  const since = $derived.by(() => {
    if (!history) return '';
    const mins = Math.max(0, Math.floor((now - Date.parse(history.startedAt)) / 60_000));
    return `Live since ${formatTime(new Date(history.startedAt))} · ${formatDuration(Math.max(60, mins * 60))}`;
  });

  function move(e: PointerEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (!r.width) return;
    const x = ((e.clientX - r.left) / r.width) * W;
    if (inBand(x, before)) {
      sel = -1;
      return;
    }
    const i = nearest(series, x, W, from);
    sel = i < 0 ? null : i;
  }

  function key(e: KeyboardEvent) {
    if (!series.length) return;
    const at = sel ?? series.length - 1;
    const low = before > 0 ? -1 : 0;
    const to =
      e.key === 'ArrowLeft' ? at - 1 : e.key === 'ArrowRight' ? at + 1 : e.key === 'Home' ? low : e.key === 'End' ? series.length - 1 : null;
    if (to === null) return;
    e.preventDefault();
    sel = Math.min(series.length - 1, Math.max(low, to));
  }

  const readout = $derived(
    sel === -1 && before > 0 ? 'Not tracked' : sel !== null && series[sel] ? `${clock(series[sel][0])} · ${formatNumber(series[sel][1])} viewers` : '',
  );
  const pct = (n: number, total: number) => (n / total) * 100;

  /** s1-10 The readout follows the pointer and stops at the plot's edges, by its own width. */
  let plotW = $state(0);
  let tipW = $state(0);
  const tipAt = (x: number) => {
    const at = (x / W) * plotW - tipW / 2;
    return Math.max(0, Math.min(plotW - tipW, at));
  };
  /** c-5 The band's label, upright when the band is wide enough for it, else on its side. */
  const bandPx = $derived((before / W) * plotW);
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
      bind:clientWidth={plotW}
    >
      <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true">
        {#if before > 0}
          <!-- c-5 Hatched, so "not tracked" reads as a gap, not as empty data. -->
          <defs>
            <pattern id="hatch-{uid}" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
              <line class="hatch" x1="0" y1="0" x2="0" y2="6" />
            </pattern>
          </defs>
          <rect class="before" x="0" y="0" width={before} height={H} />
          <rect x="0" y="0" width={before} height={H} fill="url(#hatch-{uid})" />
          <line class="edge" x1={before} y1="0" x2={before} y2={H} />
        {/if}
        <path class="area" d={g.area} />
        <polyline class="line" points={g.line} />
      </svg>
      {#if before > 0 && bandPx >= 14}
        <span class="band-label" class:side={bandPx < 64} style:width="{bandPx}px" aria-hidden="true">Not tracked</span>
      {/if}
      {#if marker}
        <span class="hair" style:left="{pct(marker[0], W)}%"></span>
        <span class="dot" style:left="{pct(marker[0], W)}%" style:top="{pct(marker[1], H)}%"></span>
        <span class="tip" bind:offsetWidth={tipW} style:left="{tipAt(marker[0])}px">{readout}</span>
      {:else if sel === -1}
        <span class="tip" bind:offsetWidth={tipW} style:left="{tipAt(before / 2)}px">{readout}</span>
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
    font-size: var(--text-sub);
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
    font-size: var(--text-hint);
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
  .hatch {
    stroke: var(--border-strong);
    stroke-width: 1.5;
  }
  .edge {
    stroke: var(--border-strong);
    stroke-dasharray: 2 2;
    vector-effect: non-scaling-stroke;
  }
  .band-label {
    position: absolute;
    left: 0;
    bottom: 4px;
    color: var(--dim);
    font-size: var(--text-hint);
    text-align: center;
    white-space: nowrap;
    pointer-events: none;
  }
  .band-label.side {
    top: 0;
    bottom: 0;
    display: grid;
    place-items: center;
    writing-mode: vertical-rl;
    transform: rotate(180deg);
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
    color: var(--text);
    font-size: var(--text-hint);
    white-space: nowrap;
    pointer-events: none;
  }
</style>
