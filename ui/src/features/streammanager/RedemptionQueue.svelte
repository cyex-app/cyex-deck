<!-- V107-R37 The redemption queue: what viewers redeemed and wait on, newest first, with
     what they typed. Complete or reject (the points go back) one, the ticked ones, or all
     shown. A new redemption in the activity feed reloads it. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import Select from '../../components/Select.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { SmRedemption } from '../../ipc/bindings/SmRedemption';
  import type { SmRedemptionStatus } from '../../ipc/bindings/SmRedemptionStatus';
  import type { SmReward } from '../../ipc/bindings/SmReward';
  import { chatIpc } from '../../ipc/chat';
  import { rewardsIpc } from '../../ipc/rewards';
  import { errorText } from '../../lib/errors';
  import { ago, formatDate, formatNumber, formatTime } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { keep, toggle } from './rewardsLogic';
  import { sm } from './streammanager.svelte';

  interface Props {
    rewards: SmReward[];
    /** Reward id to show, or '' for all. */
    filter: string;
    onfilter: (id: string) => void;
    /** Something was completed or rejected: the waiting counts changed. */
    onchanged: () => void;
  }

  let { rewards, filter, onfilter, onchanged }: Props = $props();

  let items = $state<SmRedemption[]>([]);
  let next = $state<string | null>(null);
  let loading = $state(true);
  let failed = $state('');
  let busy = $state(false);
  let picked = $state<Set<string>>(new Set());
  let now = $state(Date.now());
  let seq = 0;

  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(t);
  });

  async function load(from: string | null) {
    const mine = ++seq;
    if (!from) loading = true;
    try {
      const q = await rewardsIpc.queue(filter || null, from);
      if (mine !== seq) return;
      items = from ? [...items, ...q.items.filter((i) => !items.some((o) => o.id === i.id))] : q.items;
      next = q.next;
      picked = keep(picked, items.map((i) => i.id));
      failed = '';
    } catch (e) {
      if (mine === seq) failed = errorText(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  $effect(() => {
    void filter;
    void load(null);
  });

  // A redemption in the feed: read the queue again (quietly, a moment later).
  const lastRedemption = $derived([...sm.activity].reverse().find((a) => a.kind === 'redemption')?.id ?? null);
  let seen: string | null | undefined;
  $effect(() => {
    const id = lastRedemption;
    if (seen === undefined) {
      seen = id;
      return;
    }
    if (id === seen) return;
    seen = id;
    const t = setTimeout(() => void load(null), 1500);
    return () => clearTimeout(t);
  });

  const options = $derived([
    { value: '', label: 'All rewards' },
    ...rewards.map((r) => ({ value: r.id, label: r.waiting ? `${r.title} (${formatNumber(r.waiting)})` : r.title })),
  ]);
  const all = $derived(items.length > 0 && items.every((i) => picked.has(i.id)));
  const some = $derived(picked.size > 0 && !all);
  const targets = $derived(picked.size ? items.filter((i) => picked.has(i.id)) : items);

  async function settle(list: SmRedemption[], status: SmRedemptionStatus) {
    if (!list.length) return;
    busy = true;
    const ids = list.map((i) => i.id);
    try {
      const done = await rewardsIpc.setStatus(ids, status);
      const gone = new Set(ids.slice(0, done));
      items = items.filter((i) => !gone.has(i.id));
      picked = keep(picked, items.map((i) => i.id));
      if (done < ids.length) toast.error(`Only ${formatNumber(done)} of ${formatNumber(ids.length)} went through.`);
      else if (ids.length > 1) toast.success(status === 'FULFILLED' ? `${formatNumber(done)} completed.` : `${formatNumber(done)} rejected and refunded.`);
      onchanged();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  function when(at: string): { short: string; full: string } {
    const ms = Date.parse(at);
    if (!Number.isFinite(ms)) return { short: '', full: '' };
    const d = new Date(ms);
    return { short: ago(ms, now), full: `${formatDate(d)} ${formatTime(d)}` };
  }
</script>

<div class="line top">
  <Select label="Which reward" width="170px" value={filter} {options} onchange={onfilter} />
  <span class="grow"></span>
  <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={() => void load(null)} />
</div>
{#if items.length}
  <div class="line bulk">
    <Checkbox
      label={picked.size ? `${formatNumber(picked.size)} picked` : 'Pick all'}
      checked={all}
      indeterminate={some}
      onchange={(on) => (picked = on ? new Set(items.map((i) => i.id)) : new Set())}
    />
    <span class="grow"></span>
    <Button small disabled={busy} onclick={() => void settle(targets, 'FULFILLED')}>
      {picked.size ? 'Complete' : 'Complete all'}
    </Button>
    <Button small variant="ghost" disabled={busy} onclick={() => void settle(targets, 'CANCELED')}>
      {picked.size ? 'Reject' : 'Reject all'}
    </Button>
  </div>
{/if}

{#if loading && !items.length}
  <Loading rows={3} />
{:else if failed}
  <p class="note bad">{failed} <Button small variant="link" onclick={() => void load(null)}>Retry</Button></p>
{:else if !items.length}
  <EmptyState icon="checkCheck" text="Nothing waiting." />
{:else}
  <ul class="list">
    {#each items as r (r.id)}
      {@const t = when(r.at)}
      <li class:on={picked.has(r.id)}>
        <span class="tick"><Checkbox label="Pick {r.name ?? r.login ?? 'this'}" checked={picked.has(r.id)} onchange={() => (picked = toggle(picked, r.id))} /></span>
        <div class="main">
          <div class="head">
            {#if r.login}
              <button class="who" type="button" onclick={() => r.login && void chatIpc.openUsercard(sm.manager?.login ?? r.login, r.login)}>
                {r.name ?? r.login}
              </button>
            {:else}
              <span class="who gone">Someone</span>
            {/if}
            <span class="what">{r.reward}</span>
            <span class="at" use:tooltip={t.full}>{t.short}</span>
          </div>
          {#if r.input}<p class="input">{r.input}</p>{/if}
        </div>
        <span class="acts">
          <IconButton icon="check" label="Complete" size={14} disabled={busy} onclick={() => void settle([r], 'FULFILLED')} />
          <IconButton icon="undo" label="Reject" hint="The points go back." size={14} disabled={busy} onclick={() => void settle([r], 'CANCELED')} />
        </span>
      </li>
    {/each}
  </ul>
  {#if next}
    <Button small variant="link" onclick={() => void load(next)}>Load more</Button>
  {/if}
{/if}

<style>
  .grow {
    flex: 1;
  }
  .bulk {
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border);
  }
  .bad {
    color: var(--red) !important;
  }
  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
  }
  li {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 6px 4px;
    border-bottom: 1px solid var(--border);
    border-radius: var(--radius);
  }
  li.on {
    background: var(--row-hover);
  }
  /* The box only; the name stays for screen readers. */
  .tick :global(.check) {
    gap: 0;
    font-size: 0;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    font-size: 0.82rem;
  }
  .who {
    flex: none;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-weight: 700;
    cursor: pointer;
  }
  .who:hover {
    text-decoration: underline;
  }
  .who.gone {
    color: var(--muted);
    cursor: default;
  }
  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .at {
    flex: none;
    color: var(--dim);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .input {
    margin: 0;
    font-size: 0.85rem;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .acts {
    display: flex;
    flex: none;
  }
</style>
