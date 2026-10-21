<!-- V107-R37 The redemption queue: what viewers redeemed and wait on, newest first, with
     what they typed. Complete or reject (the points go back) one, the ticked ones, or all
     shown. A new redemption in the activity feed reloads it. The reward filter and Refresh
     sit in the Rewards panel's toolbar and header. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import type { SmRedemption } from '../../ipc/bindings/SmRedemption';
  import type { SmRedemptionStatus } from '../../ipc/bindings/SmRedemptionStatus';
  import { chatIpc } from '../../ipc/chat';
  import { rewardsIpc } from '../../ipc/rewards';
  import { smIpc } from '../../ipc/streammanager';
  import { everyShown } from '../../lib/awake';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { PagedList } from '../../lib/paged.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { bulkLines } from './content';
  import LoadMore from './LoadMore.svelte';
  import PickBar from './PickBar.svelte';
  import PickBox from './PickBox.svelte';
  import SmProblem from './SmProblem.svelte';
  import { sm } from './streammanager.svelte';
  import When from './When.svelte';

  interface Props {
    /** Reward id to show, or '' for all. */
    filter: string;
    /** Something was completed or rejected: the waiting counts changed. */
    onchanged: () => void;
  }

  let { filter, onchanged }: Props = $props();

  let busy = $state(false);
  let now = $state(Date.now());

  // V107-H1 Not while hidden; once when it shows.
  $effect(() => everyShown(() => (now = Date.now()), 30_000));

  const list = new PagedList<SmRedemption>({ key: (r) => r.id, fetch: (cursor) => rewardsIpc.queue(filter || null, cursor) });
  const items = $derived(list.items);
  const picked = $derived(list.picked);

  /** The header's Refresh. */
  export function reload() {
    void list.reload();
  }

  $effect(() => {
    void filter;
    untrack(() => void list.reload({ clear: true }));
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
    const t = setTimeout(() => void list.reload(), 1500);
    return () => clearTimeout(t);
  });

  const targets = $derived(picked.size ? list.pickedItems() : items);
  /** BUG-05 More pages wait: "all" would only be the ones shown. */
  const all = $derived(list.next ? 'shown' : 'all');
  const ask = (verb: string) => (list.next ? `${verb} ${formatNumber(items.length)} shown?` : `${verb} all ${formatNumber(items.length)}?`);

  async function settle(rows: SmRedemption[], status: SmRedemptionStatus) {
    if (!rows.length || busy) return;
    busy = true;
    const ids = rows.map((i) => i.id);
    try {
      const r = await rewardsIpc.setStatus(ids, status);
      list.remove(r.done);
      const say = bulkLines(r, ids.length, (n) =>
        ids.length < 2 ? '' : status === 'FULFILLED' ? `${formatNumber(n)} completed.` : `${formatNumber(n)} rejected and refunded.`,
      );
      if (say.ok) toast.success(say.ok);
      if (say.info) toast.info(say.info);
      if (say.bad) toast.error(say.bad);
      onchanged();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }
</script>

{#if items.length}
  <PickBar picked={picked.size} total={items.length} onall={list.pickAll}>
    {#if busy && targets.length > 1}
      <Button small variant="ghost" onclick={() => void smIpc.bulkStop()}>Stop</Button>
    {:else if picked.size || items.length === 1}
      <Button small disabled={busy} onclick={() => void settle(targets, 'FULFILLED')}>Complete</Button>
      <Button small variant="ghost" disabled={busy} onclick={() => void settle(targets, 'CANCELED')}>Reject</Button>
    {:else}
      <ConfirmButton
        label="Complete {all}"
        confirm={ask('Complete')}
        disabled={busy}
        onconfirm={() => void settle(targets, 'FULFILLED')}
      />
      <ConfirmButton
        label="Reject {all}"
        confirm={ask('Reject')}
        variant="ghost"
        disabled={busy}
        onconfirm={() => void settle(targets, 'CANCELED')}
      />
    {/if}
  </PickBar>
{/if}

{#if list.loading && !items.length}
  <Loading rows={5} label="Loading the queue" />
{:else if list.failed && !items.length}
  <SmProblem text={list.failed} onretry={() => void list.reload()} />
{:else if !items.length}
  <EmptyState icon="checkCheck" text="Nothing waiting." />
{:else}
  <ul class="list">
    {#each items as r (r.id)}
      <li class:on={picked.has(r.id)}>
        <PickBox label="Pick {r.name ?? r.login ?? 'this'}" checked={picked.has(r.id)} onchange={() => list.toggle(r.id)} />
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
            <When at={r.at} {now} short />
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
  {#if list.failed}<p class="note bad">{list.failed}</p>{/if}
  {#if list.next}<LoadMore busy={list.more} onclick={() => void list.loadMore()} />{/if}
{/if}

<style>
  .bad {
    padding: 4px 12px;
    color: var(--red) !important;
  }
  .list {
    margin: 0;
    padding: 4px;
    list-style: none;
    display: flex;
    flex-direction: column;
  }
  li {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px;
    border-radius: var(--radius);
  }
  li:hover {
    background: var(--row-hover);
  }
  li.on {
    background: var(--surface-hover);
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
    font-size: 0.85rem;
  }
  .who {
    flex: none;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-weight: 600;
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
