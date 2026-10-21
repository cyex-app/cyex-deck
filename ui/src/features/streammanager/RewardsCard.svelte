<!-- V107-R37 Channel points rewards on your own channel (Twitch's Viewer Rewards › Channel
     Points) and the redemption queue. Rewards: on/off, pause, edit, delete with an inline
     confirm, new. Queue: see RedemptionQueue. GQL only, so every reward shows, not just
     the ones this app made. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import Card from '../../components/PanelCard.svelte';
  import Segmented from '../../components/Segmented.svelte';
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
  import { blank, facts, toEdit, waiting } from './rewardsLogic';

  type View = 'rewards' | 'queue';
  const VIEWS = [
    { value: 'rewards', label: 'Rewards' },
    { value: 'queue', label: 'Queue' },
  ] as const;

  let view = $state<View>('rewards');
  let filter = $state('');
  let data = $state<SmRewards | null>(null);
  let loading = $state(true);
  let failed = $state('');
  /** `new`, a reward id being edited, or null. */
  let editing = $state<string | null>(null);
  let confirming = $state<string | null>(null);
  let busy = $state<string | null>(null);

  const rewards = $derived(data?.rewards ?? []);
  const queued = $derived(rewards.reduce((n, r) => n + r.waiting, 0));

  async function load() {
    try {
      data = await rewardsIpc.list();
      failed = '';
    } catch (e) {
      failed = errorText(e);
    } finally {
      loading = false;
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

  async function save(edit: SmRewardEdit) {
    const id = editing;
    busy = id ?? 'new';
    try {
      put(id && id !== 'new' ? await rewardsIpc.update(id, edit) : await rewardsIpc.create(edit));
      toast.success(id && id !== 'new' ? 'Reward saved.' : 'Reward created.');
      editing = null;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = null;
    }
  }

  async function flip(r: SmReward, change: SmRewardSwitch) {
    busy = r.id;
    try {
      put(await rewardsIpc.switch(r.id, change));
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = null;
    }
  }

  async function remove(r: SmReward, waitingTo: SmRedemptionStatus | null) {
    busy = r.id;
    try {
      await rewardsIpc.remove(r.id, waitingTo);
      if (data) data.rewards = data.rewards.filter((x) => x.id !== r.id);
      if (filter === r.id) filter = '';
      toast.success('Reward deleted.');
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = null;
      confirming = null;
    }
  }

  function openQueue(id: string) {
    filter = id;
    view = 'queue';
  }
</script>

<Card title="Rewards">
  {#if data?.enabled}
    <div class="line">
      <Segmented options={VIEWS} value={view} label="Rewards or queue" neutral onchange={(v) => (view = v)} />
      <span class="grow"></span>
      {#if view === 'rewards'}
        <Button small icon="plus" disabled={editing === 'new'} onclick={() => (editing = 'new')}>New</Button>
      {/if}
    </div>
  {/if}
  {#if loading}
    <Loading rows={3} />
  {:else if failed}
    <p class="note bad">{failed} <Button small variant="link" onclick={() => void load()}>Retry</Button></p>
  {:else if !data?.enabled}
    <EmptyState icon="points" text="Channel points are off for your channel. They come with Affiliate." />
  {:else if view === 'queue'}
    <RedemptionQueue {rewards} {filter} onfilter={(id) => (filter = id)} onchanged={() => void load()} />
  {:else}
    {#if queued}
      <p class="note">
        <Button small variant="link" onclick={() => openQueue('')}>{formatNumber(queued)} waiting in the queue</Button>
      </p>
    {/if}
    {#if editing === 'new'}
      <RewardForm start={blank()} editing={false} busy={busy === 'new'} onsave={save} oncancel={() => (editing = null)} />
    {/if}
    {#if !rewards.length && editing !== 'new'}
      <EmptyState icon="points" text="No rewards yet.">
        <Button small onclick={() => (editing = 'new')}>New reward</Button>
      </EmptyState>
    {/if}
    <ul class="list">
      {#each rewards as r (r.id)}
        {@const f = facts(r)}
        {@const w = waiting(r)}
        <li class:off={!r.enabled}>
          {#if editing === r.id}
            <RewardForm start={toEdit(r)} editing busy={busy === r.id} onsave={save} oncancel={() => (editing = null)} />
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
                  <Toggle
                    label="Reward on"
                    checked={r.enabled}
                    disabled={busy === r.id}
                    onchange={(on) => void flip(r, { kind: 'enabled', on })}
                  />
                  <span class="quiet" class:shown={r.paused}><IconButton
                    icon={r.paused ? 'play' : 'pause'}
                    label={r.paused ? 'Resume' : 'Pause'}
                    hint={r.paused ? 'Viewers can redeem it again.' : 'Viewers see it but can’t redeem it.'}
                    size={14}
                    disabled={busy === r.id || !r.enabled}
                    onclick={() => void flip(r, { kind: 'paused', on: !r.paused })}
                  /></span>
                  <span class="quiet"><IconButton icon="pencil" label="Edit" size={14} disabled={busy === r.id} onclick={() => (editing = r.id)} />
                  <IconButton icon="trash" label="Delete" size={14} disabled={busy === r.id} onclick={() => (confirming = r.id)} /></span>
                </span>
              {/if}
            </div>
            {#if confirming === r.id}
              <div class="line confirm">
                {#if r.waiting}
                  <span class="ask">Delete it? {formatNumber(r.waiting)}{r.waitingMore ? '+' : ''} still waiting.</span>
                  <Button small variant="danger" disabled={busy === r.id} onclick={() => void remove(r, 'CANCELED')}>Delete and refund</Button>
                  <Button small disabled={busy === r.id} onclick={() => void remove(r, 'FULFILLED')}>Delete and complete</Button>
                {:else}
                  <span class="ask">Delete it?</span>
                  <Button small variant="danger" disabled={busy === r.id} onclick={() => void remove(r, null)}>Delete</Button>
                {/if}
                <Button small variant="ghost" onclick={() => (confirming = null)}>Cancel</Button>
              </div>
            {/if}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</Card>

<style>
  .grow {
    flex: 1;
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
    gap: 2px;
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 2px;
    border-bottom: 1px solid var(--border);
  }
  li:last-child {
    border-bottom: 0;
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
    overflow-wrap: anywhere;
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
  /* Quiet until the row is hovered or reached by keyboard; a paused reward shows Resume. */
  .quiet {
    display: flex;
    opacity: 0;
    transition: opacity var(--fast);
  }
  li:hover .quiet,
  .quiet:focus-within,
  .quiet.shown {
    opacity: 1;
  }
  .confirm {
    padding-left: 36px;
  }
  .ask {
    font-size: 0.8rem;
  }
</style>
