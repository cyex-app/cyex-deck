<!-- 1.32 Miner stats: pick a channel (A-Z) or all of them; points this session and in
     total, the balance over time, where the points came from, bets and drops. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { StatKind } from '../../ipc/bindings/StatKind';
  import type { Stats } from '../../ipc/bindings/Stats';
  import type { StatsChannel } from '../../ipc/bindings/StatsChannel';
  import { minerIpc } from '../../ipc/miner';
  import { formatDate, formatNumber } from '../../lib/format';
  import { latest } from '../../lib/latest';
  import { toast } from '../../lib/toast.svelte';
  import { miner } from './miner.svelte';
  import { errorText } from '../../lib/errors';
  import Select from '../../components/Select.svelte';

  const KINDS: Record<StatKind, string> = {
    watch: 'Watching',
    chest: 'Bonus chests',
    streak: 'Watch streaks',
    raid: 'Raids',
    moment: 'Moments',
    bet: 'Bets placed',
    won: 'Bets won',
    lost: 'Bets lost',
    refund: 'Bets refunded',
    drop: 'Drops',
    goal: 'Community goals',
    watched: 'Watched',
    other: 'Other',
  };

  let channels = $state.raw<StatsChannel[]>([]);
  let channel = $state<string | null>(null);
  let stats = $state.raw<Stats | null>(null);
  let confirm = $state(false);

  const signed = (n: number) => `${n > 0 ? '+' : n < 0 ? '−' : ''}${formatNumber(Math.abs(n))}`;
  const hours = (m: number) => (m >= 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : `${m} min`);

  const seq = latest();
  /** The channel `stats` belongs to. */
  let shown: string | null = null;

  async function load() {
    const pick = channel;
    const live = seq.begin();
    if (pick !== shown) stats = null;
    try {
      const [list, s] = await Promise.all([minerIpc.statsChannels(), minerIpc.stats(pick)]);
      if (!live()) return;
      channels = list;
      stats = s;
      shown = pick;
    } catch (e) {
      if (live()) toast.error(errorText(e));
    }
  }

  // Again when the channel changes, and as the log grows.
  $effect(() => {
    void channel;
    void miner.log.length;
    void load();
  });

  async function clear() {
    if (!confirm) {
      confirm = true;
      return;
    }
    confirm = false;
    try {
      await minerIpc.clearStats(channel);
    } catch (e) {
      return toast.error(errorText(e));
    }
    channel = null;
    await load();
  }

  /** The balance as an SVG path, 600×120. */
  const chart = $derived.by(() => {
    const pts = stats?.series ?? [];
    if (pts.length < 2) return null;
    const [t0, t1] = [pts[0][0], pts[pts.length - 1][0]];
    const values = pts.map((p) => p[1]);
    const [lo, hi] = [Math.min(...values), Math.max(...values)];
    const x = (t: number) => ((t - t0) / Math.max(1, t1 - t0)) * 600;
    const y = (v: number) => 112 - ((v - lo) / Math.max(1, hi - lo)) * 104;
    return {
      d: pts.map((p, i) => `${i ? 'L' : 'M'}${x(p[0]).toFixed(1)},${y(p[1]).toFixed(1)}`).join(''),
      lo,
      hi,
      from: formatDate(new Date(t0)),
      to: formatDate(new Date(t1)),
    };
  });
</script>

<div class="stats">
  <div class="top">
    <Select
      label="Channel"
      width="240px"
      value={channel ?? ''}
      options={[{ value: '', label: 'All channels' }, ...channels.map((c) => ({ value: c.id, label: c.name }))]}
      onchange={(v) => (channel = v || null)}
    />
    {#if stats?.since}<span class="note">Since {formatDate(new Date(stats.since))}</span>{/if}
    <span class="grow"></span>
    {#if stats?.since}
      <Button variant="ghost" onclick={() => void clear()}>{confirm ? 'Sure? Delete them' : 'Delete stats'}</Button>
    {/if}
  </div>

  {#if !stats?.since}
    <p class="empty">Nothing yet. Stats fill in while the miner runs.</p>
  {:else}
    <div class="cards">
      {#if stats.balance != null && channel}
        <div class="card"><span class="label">Balance</span><b>{formatNumber(stats.balance)}</b></div>
      {/if}
      <div class="card"><span class="label">This session</span><b>{signed(stats.session)}</b></div>
      <div class="card"><span class="label">In total</span><b>{signed(stats.total)}</b></div>
      {#if stats.minutes}
        <div class="card"><span class="label">Watched</span><b>{hours(stats.minutes)}</b></div>
      {/if}
      {#if stats.betsWon || stats.betsLost}
        <div class="card"><span class="label">Bets won</span><b>{stats.betsWon} of {stats.betsWon + stats.betsLost}</b></div>
      {/if}
    </div>

    {#if chart && channel}
      <figure class="chart">
        <svg viewBox="0 0 600 120" preserveAspectRatio="none" aria-label="Balance over time">
          <path d={chart.d} />
        </svg>
        <figcaption>
          <span>{chart.from}</span>
          <span>{formatNumber(chart.lo)} to {formatNumber(chart.hi)}</span>
          <span>{chart.to}</span>
        </figcaption>
      </figure>
    {/if}

    <h2>Where the points came from</h2>
    <table>
      <tbody>
        {#each stats.kinds as k (k.kind)}
          <tr>
            <td>{KINDS[k.kind]}</td>
            <td class="num">{k.count}×</td>
            <td class="num">{signed(k.points)}</td>
          </tr>
        {/each}
      </tbody>
    </table>

    {#if stats.drops.length}
      <h2>Drops claimed</h2>
      {#each stats.drops as [t, name] (t + name)}
        <p class="drop"><span class="when">{formatDate(new Date(t))}</span>{name}</p>
      {/each}
    {/if}
  {/if}
</div>

<style>
  .stats {
    max-width: 760px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .grow {
    flex: 1;
  }
  .note,
  .empty {
    color: var(--dim);
    font-size: 0.82rem;
  }
  .cards {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 14px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 120px;
    padding: 10px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .label {
    color: var(--dim);
    font-size: 0.75rem;
  }
  .card b {
    font-size: 1.05rem;
    font-variant-numeric: tabular-nums;
  }
  .chart {
    margin: 16px 0 0;
  }
  .chart svg {
    display: block;
    width: 100%;
    height: 120px;
    background: var(--surface);
    border-radius: var(--radius);
  }
  .chart path {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
  }
  figcaption {
    display: flex;
    justify-content: space-between;
    margin-top: 4px;
    color: var(--dim);
    font-size: 0.75rem;
  }
  h2 {
    margin: 18px 0 6px;
    font-size: 0.85rem;
    color: var(--muted);
  }
  table {
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  td {
    padding: 3px 18px 3px 0;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .drop {
    margin: 2px 0;
    font-size: 0.85rem;
  }
  .when {
    margin-right: 10px;
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
</style>
