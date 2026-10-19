<!-- 1.35 Stream preview and stats: Twitch's preview picture while live, viewers, uptime,
     followers, subs and sub points, and the X4 viewer graph for this session. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import { formatNumber } from '../../lib/format';
  import { preview, uptimeShort } from '../browse/lists';
  import Card from '../../components/PanelCard.svelte';
  import { graph } from '../../lib/sparkline';
  import { sm } from './streammanager.svelte';
  import Loading from '../../components/Loading.svelte';
  import { everyShown } from '../../lib/awake';

  let now = $state(Date.now());
  // V107-H1 No new picture while the window is hidden; one when it shows.
  $effect(() => everyShown(() => (now = Date.now()), 15_000));

  const s = $derived(sm.stats);
  const m = $derived(sm.manager);
  const g = $derived(graph(sm.series, 300, 64));
  const n = (v: number | null | undefined) => (v == null ? '—' : formatNumber(v));
</script>

<Card title="Stream">
  {#if m}
    {#if s?.live}
      <img class="preview" src={preview(m.login, 440, 248, now)} alt="Your stream" />
    {:else}
      <div class="preview off">
        <Avatar src={m.avatar} name={m.display} size={40} />
        {#if s}<span>Offline</span>{:else}<Loading />{/if}
      </div>
    {/if}
  {/if}
  <dl class="stats">
    <div><dt>Viewers</dt><dd>{s?.live ? formatNumber(s.viewers) : '—'}</dd></div>
    <div><dt>Live for</dt><dd>{s?.live ? uptimeShort(s.startedAt, now) : '—'}</dd></div>
    <div><dt>Followers</dt><dd>{n(s?.followers)}</dd></div>
    <div><dt>Subs</dt><dd>{n(s?.subs)}</dd></div>
    <div><dt>Sub points</dt><dd>{n(s?.subPoints)}</dd></div>
  </dl>
  {#if g.line}
    <figure class="graph">
      <svg viewBox="0 0 300 64" preserveAspectRatio="none" role="img" aria-label="Viewers over time">
        <path class="area" d={g.area} />
        <polyline class="line" points={g.line} />
      </svg>
      <figcaption>
        <span>Viewers this session</span>
        <span>{formatNumber(g.min)}–{formatNumber(g.max)}</span>
      </figcaption>
    </figure>
  {:else}
    <p class="note">The viewer graph fills in while you’re live.</p>
  {/if}
</Card>

<style>
  .preview {
    width: 100%;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    border-radius: var(--radius);
    background: var(--bg);
  }
  .preview.off {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
    gap: 6px;
    margin: 0;
  }
  .stats div {
    padding: 6px 8px;
    background: var(--bg);
    border-radius: var(--radius);
  }
  dt {
    font-size: 0.7rem;
    color: var(--dim);
  }
  dd {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .graph {
    margin: 0;
  }
  svg {
    display: block;
    width: 100%;
    height: 64px;
  }
  .area {
    fill: var(--accent-dim);
  }
  .line {
    fill: none;
    stroke: var(--accent-text);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }
  figcaption {
    display: flex;
    flex-wrap: wrap;
    gap: 0 8px;
    justify-content: space-between;
    font-size: 0.72rem;
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
</style>
