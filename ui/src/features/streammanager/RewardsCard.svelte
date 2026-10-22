<!-- V107-R37 Channel points rewards on your own channel (Twitch's Viewer Rewards › Channel
     Points) and the redemption queue. Rewards: on/off, pause, edit, delete with an inline
     confirm, new. Queue: see RedemptionQueue. GQL only, so every reward shows, not just
     the ones this app made. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import ConfirmChoices from '../../components/ConfirmChoices.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import Loading from '../../components/Loading.svelte';
  import Card from '../../components/PanelCard.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import Select from '../../components/Select.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import type { SmRedemptionStatus } from '../../ipc/bindings/SmRedemptionStatus';
  import type { SmReward } from '../../ipc/bindings/SmReward';
  import type { SmRewardEdit } from '../../ipc/bindings/SmRewardEdit';
  import type { SmRewards } from '../../ipc/bindings/SmRewards';
  import type { SmRewardSwitch } from '../../ipc/bindings/SmRewardSwitch';
  import { rewardsIpc } from '../../ipc/rewards';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import RedemptionQueue from './RedemptionQueue.svelte';
  import RewardForm from './RewardForm.svelte';
  import { blank, facts, rewardOptions, toEdit, waiting } from './rewardsLogic';
  import { focusBack } from './onEscape';
  import SmBar from './SmBar.svelte';
  import SmProblem from './SmProblem.svelte';

  type View = 'rewards' | 'queue';

  let view = $state<View>('rewards');
  let filter = $state('');
  let data = $state<SmRewards | null>(null);
  let loading = $state(true);
  let failed = $state('');
  /** `new`, a reward id being edited, or null. */
  let editing = $state<string | null>(null);
  let confirming = $state<string | null>(null);
  /** BUG-06 Rows (or `new`) with a write on the way; others stay usable. */
  let busy = $state<ReadonlySet<string>>(new Set());
  const isBusy = (id: string) => busy.has(id);
  /** Runs one row's write, that row held meanwhile. */
  async function hold(id: string, run: () => Promise<void>) {
    if (busy.has(id)) return;
    busy = new Set([...busy, id]);
    try {
      await run();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = new Set([...busy].filter((x) => x !== id));
    }
  }

  const rewards = $derived(data?.rewards ?? []);
  const queued = $derived(rewards.reduce((n, r) => n + r.waiting, 0));
  const views = $derived<{ value: View; label: string }[]>([
    { value: 'rewards', label: 'Rewards' },
    { value: 'queue', label: queued ? `Queue ${formatNumber(queued)}` : 'Queue' },
  ]);
  const options = $derived(rewardOptions(rewards));
  let queue = $state<RedemptionQueue>();

  function refresh() {
    if (view === 'queue') queue?.reload();
    void load();
  }


  /** BUG-06 Only the newest read lands. */
  let seq = 0;
  async function load() {
    const mine = ++seq;
    loading = true;
    try {
      const r = await rewardsIpc.list();
      if (mine !== seq) return;
      data = r;
      failed = '';
    } catch (e) {
      if (mine !== seq) return;
      failed = errorText(e);
      // V107-N4C The list stays up: say the refresh failed, or it looks like it worked.
      if (data) toast.error(failed);
    } finally {
      if (mine === seq) loading = false;
    }
  }
  void load();

  function put(r: SmReward) {
    if (!data) return;
    const i = data.rewards.findIndex((x) => x.id === r.id);
    const old = i >= 0 ? data.rewards[i] : null;
    // The waiting count isn't in a save's answer.
    const next = old ? { ...r, waiting: old.waiting, waitingMore: old.waitingMore } : r;
    data.rewards = i >= 0 ? data.rewards.map((x, j) => (j === i ? next : x)) : [...data.rewards, next];
  }

  function save(edit: SmRewardEdit) {
    const id = editing ?? 'new';
    void hold(id, async () => {
      if (id !== 'new') {
        put(await rewardsIpc.update(id, edit));
        toast.success('Reward saved.');
      } else {
        const s = await rewardsIpc.create(edit);
        put(s.reward);
        if (s.warning) toast.error(s.warning);
        else toast.success('Reward created.');
      }
      if (editing === id) editing = null;
    });
  }

  function flip(r: SmReward, change: SmRewardSwitch) {
    void hold(r.id, async () => put(await rewardsIpc.switch(r.id, change)));
  }

  function remove(r: SmReward, waitingTo: SmRedemptionStatus | null) {
    void hold(r.id, async () => {
      try {
        await rewardsIpc.remove(r.id, waitingTo);
        if (data) data.rewards = data.rewards.filter((x) => x.id !== r.id);
        if (filter === r.id) filter = '';
        toast.success('Reward deleted.');
      } finally {
        if (confirming === r.id) confirming = null;
      }
    });
  }

  /** Pause, edit and delete sit behind ⋯, as in Clips and Videos; the switch stays out. */
  function menu(e: MouseEvent, r: SmReward) {
    openMenuAt(e, [
      r.paused
        ? { label: 'Resume', icon: 'play', note: 'Viewers can redeem it again.', run: () => void flip(r, { kind: 'paused', on: false }) }
        : { label: 'Pause', icon: 'pause', disabled: !r.enabled, note: 'Viewers see it but can’t redeem it.', run: () => void flip(r, { kind: 'paused', on: true }) },
      { label: 'Edit', icon: 'pencil', run: () => (editing = r.id) },
      'separator',
      { label: 'Delete reward', icon: 'trash', danger: true, run: () => (confirming = r.id) },
    ]);
  }

  /** Out of the delete confirm, focus back on the row's ⋯. */
  function back(id: string) {
    confirming = null;
    focusBack(id);
  }

  function openQueue(id: string) {
    filter = id;
    view = 'queue';
  }
</script>

<Card title="Rewards" flush>
  {#snippet tools()}
    <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={refresh} />
    {#if data?.enabled && view === 'rewards'}
      <IconButton icon="plus" label="New reward" size={14} active={editing === 'new'} toggle onclick={() => (editing = editing === 'new' ? null : 'new')} />
    {/if}
  {/snippet}
  {#if data?.enabled}
    <SmBar>
      <div class="filters">
        <Segmented options={views} value={view} label="Rewards or queue" onchange={(v) => (view = v)} />
        {#if view === 'rewards'}<span class="count">{formatNumber(rewards.length)} {rewards.length === 1 ? 'reward' : 'rewards'}</span>{/if}
      </div>
      {#if view === 'queue'}<Select label="Which reward" width="100%" value={filter} {options} onchange={(id) => (filter = id)} />{/if}
    </SmBar>
  {/if}
  {#if loading && !data}
    <Loading rows={5} label="Loading rewards" />
  {:else if failed && !data}
    <SmProblem text={failed} onretry={() => void load()} />
  {:else if !data?.enabled}
    <EmptyState icon="points" text="Channel points are off for your channel. They come with Affiliate." />
  {:else if view === 'queue'}
    <RedemptionQueue bind:this={queue} {filter} onchanged={() => void load()} />
  {:else}
    {#if editing === 'new'}
      <div class="new"><RewardForm start={blank()} editing={false} busy={isBusy('new')} onsave={save} oncancel={() => (editing = null)} /></div>
    {/if}
    {#if !rewards.length && editing !== 'new'}
      <EmptyState icon="points" text="No rewards yet.">
        <Button small icon="plus" onclick={() => (editing = 'new')}>New reward</Button>
      </EmptyState>
    {/if}
    <ul class="list">
      {#each rewards as r (r.id)}
        {@const f = facts(r)}
        {@const w = waiting(r)}
        <li class:off={!r.enabled}>
          {#if editing === r.id}
            <RewardForm start={toEdit(r)} editing busy={isBusy(r.id)} onsave={save} oncancel={() => (editing = null)} />
          {:else}
            <div class="row">
              <span class="icon" style:background={r.color}>
                {#if r.image}<img src={r.image} alt="" />{/if}
              </span>
              <div class="main">
                <div class="head">
                  <span class="title">{r.title}</span>
                  <span class="cost"><Icon name="points" size={11} />{formatNumber(r.cost)}</span>
                </div>
                {#if r.prompt}<p class="prompt">{r.prompt}</p>{/if}
                {#if f.length || w}
                  <p class="facts">
                    {f.join(' · ')}{#if w}{f.length ? ' · ' : ''}<button class="wait" type="button" onclick={() => openQueue(r.id)}>{w}</button>{/if}
                  </p>
                {/if}
              </div>
              {#if confirming !== r.id}
                <span class="acts">
                  <Toggle label="Reward on" checked={r.enabled} disabled={isBusy(r.id)} onchange={(on) => flip(r, { kind: 'enabled', on })} />
                  <span class="more" data-back={r.id}><IconButton icon="more" label="More for {r.title}" size={14} disabled={isBusy(r.id)} onclick={(e) => menu(e, r)} /></span>
                </span>
              {/if}
            </div>
            {#if confirming === r.id}
              <div class="confirm">
                <ConfirmChoices
                  ask={r.waiting ? `Delete it? ${formatNumber(r.waiting)}${r.waitingMore ? '+' : ''} still waiting.` : 'Delete it?'}
                  choices={r.waiting
                    ? [
                        { label: 'Delete and refund', variant: 'danger', run: () => remove(r, 'CANCELED') },
                        { label: 'Delete and complete', run: () => remove(r, 'FULFILLED') },
                      ]
                    : [{ label: 'Delete', variant: 'danger', run: () => remove(r, null) }]}
                  disabled={isBusy(r.id)}
                  oncancel={() => back(r.id)}
                />
              </div>
            {/if}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</Card>

<style>
  .new {
    padding: 8px;
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
    flex-direction: column;
    gap: 6px;
    padding: 6px;
    border-radius: var(--radius);
  }
  li:hover {
    background: var(--row-hover);
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  li.off .icon,
  li.off .main {
    opacity: 0.55;
  }
  .icon {
    flex: none;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: var(--radius);
  }
  .icon img {
    width: 20px;
    height: 20px;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 0.85rem;
  }
  .title {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    overflow-wrap: break-word;
  }
  .cost {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--muted);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .prompt,
  .facts {
    margin: 0;
    font-size: 0.75rem;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .facts {
    color: var(--dim);
  }
  .wait {
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .wait:hover {
    text-decoration: underline;
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }
  .confirm {
    padding-left: 36px;
  }
  .more {
    display: flex;
  }
</style>
