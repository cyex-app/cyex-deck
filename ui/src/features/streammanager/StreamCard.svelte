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
  import { settings } from '../../lib/settings.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { isHidden, toggleStat, type StatId } from './statsHidden.ts';

  let now = $state(Date.now());
  // V107-H1 No new picture while the window is hidden; one when it shows.
  $effect(() => everyShown(() => (now = Date.now()), 15_000));

  const s = $derived(sm.stats);
  const m = $derived(sm.manager);
  const g = $derived(graph(sm.series, 300, 64));
  // V108-C38 A click hides a number (dots, same width) or shows it again; remembered.
  const hidden = $derived(settings.v.streammanager.hiddenStats ?? []);
  const flip = (id: StatId) => void settings.patch('streammanager', { hiddenStats: toggleStat(hidden, id) });
  const n = (v: number | null | undefined) => (v == null ? '—' : formatNumber(v));
  const stats = $derived<{ id: StatId; label: string; text: string }[]>([
    { id: 'viewers', label: 'Viewers', text: s?.live ? formatNumber(s.viewers) : '—' },
    { id: 'uptime', label: 'Live for', text: s?.live ? uptimeShort(s.startedAt, now) : '—' },
    { id: 'followers', label: 'Followers', text: n(s?.followers) },
    { id: 'subs', label: 'Subs', text: n(s?.subs) },
    { id: 'subPoints', label: 'Sub points', text: n(s?.subPoints) },
  ]);
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
    {#each stats as st (st.id)}
      {@const off = isHidden(hidden, st.id)}
      <div>
        <dt>{st.label}</dt>
        <dd>
          <button
            type="button"
            class="val"
            class:off
            aria-pressed={off}
            aria-label="{st.label}, {off ? 'hidden' : 'shown'}"
            use:tooltip={off ? 'Show' : 'Hide'}
            onclick={() => flip(st.id)}
          >{off ? '•••' : st.text}</button>
        </dd>
      </div>
    {/each}
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
  .val {
    all: unset;
    box-sizing: border-box;
    display: block;
    min-width: 100%;
    cursor: pointer;
    border-radius: 2px;
  }
  .val.off {
    color: var(--dim);
    letter-spacing: 0.15em;
  }
  .val:focus-visible {
    outline: 2px solid var(--accent-text);
    outline-offset: 2px;
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
