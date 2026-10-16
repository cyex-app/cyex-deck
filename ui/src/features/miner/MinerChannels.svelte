<!-- The miner's channels (1.31, 1.33, 1.44, W3): the two mined first, then your list
     (drag to reorder) or your follows (skip the ones you don't want), and drops still
     being earned. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import DragList from '../../components/DragList.svelte';
  import Icon from '../../components/Icon.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
  import type { MineSource } from '../../ipc/bindings/MineSource';
  import type { TabChannel } from '../../ipc/bindings/TabChannel';
  import { minerIpc } from '../../ipc/miner';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import ChannelPicker from '../tabs/ChannelPicker.svelte';
  import { miner } from './miner.svelte';
  import MinerRow from './MinerRow.svelte';
  import { errorText } from '../../lib/errors';

  const SOURCES: { value: MineSource; label: string }[] = [
    { value: 'list', label: 'Your list' },
    { value: 'follows', label: 'Your follows' },
  ];

  const s = $derived(settings.v.miner);
  const idle = $derived(!miner.status?.running);
  const info = $derived(new Map((miner.status?.channels ?? []).map((c) => [c.id, c])));
  const firstIds = $derived(new Set(s.first.map((c) => c.id)));
  const list = $derived(s.channels.filter((c) => !firstIds.has(c.id)));
  const skipIds = $derived(new Set(s.skip.map((c) => c.id)));
  const follows = $derived(
    (miner.status?.channels ?? [])
      .filter((c) => !firstIds.has(c.id) && !skipIds.has(c.id))
      .map((c): TabChannel => ({ id: c.id, login: c.login, name: c.name })),
  );

  let text = $state('');
  let picked = $state<ChannelInfo | null>(null);

  const strip = (c: { id: string; login: string; name: string }): TabChannel => ({
    id: c.id,
    login: c.login,
    name: c.name,
  });

  async function add() {
    if (!picked) return;
    const c = strip(picked);
    try {
      if (!(await minerIpc.add(c))) toast.info(`${c.name} is on the list already.`);
    } catch (e) {
      // Kept in the box to try again.
      return toast.error(errorText(e));
    }
    text = '';
    picked = null;
  }

  async function setFirst(c: TabChannel, on: boolean) {
    try {
      const out = await minerIpc.first(strip(c), on);
      if (out) toast.info(`${out} isn’t first anymore.`);
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function remove(id: string) {
    void settings.patch('miner', {
      channels: s.channels.filter((c) => c.id !== id),
      first: s.first.filter((c) => c.id !== id),
    });
  }

  function skip(c: TabChannel, on: boolean) {
    void settings.patch('miner', { skip: on ? [...s.skip, strip(c)] : s.skip.filter((x) => x.id !== c.id) });
  }

  /** V105-14 The list dragged into a new order; the first two stay where they are. */
  function reorder(next: TabChannel[]) {
    void settings.patch('miner', { channels: [...s.channels.filter((c) => firstIds.has(c.id)), ...next] });
  }
</script>

<div class="channels">
  <div class="top">
    <Segmented
      label="What to mine"
      options={SOURCES}
      value={s.source}
      onchange={(source) => void settings.patch('miner', { source })}
    />
    <span class="note">Twitch counts two channels at a time. The miner takes them from the top.</span>
  </div>

  {#if s.first.length}
    <h2><Icon name="starFilled" size={13} />First</h2>
    {#each s.first as c (c.id)}
      <MinerRow channel={c} info={info.get(c.id)} {idle} first onfirst={(on) => void setFirst(c, on)} />
    {/each}
  {/if}

  {#if s.source === 'list'}
    <h2>Your list <span class="count">{list.length}</span></h2>
    <div class="add">
      <div class="picker"><ChannelPicker bind:text bind:picked onsubmit={() => void add()} /></div>
      <Button disabled={!picked} onclick={() => void add()}>Add</Button>
    </div>
    {#if list.length}
      <DragList key="miner" items={list} id={(c) => c.id} label={(c) => c.name} onmove={reorder}>
        {#snippet row(c)}
          <MinerRow
            channel={c}
            info={info.get(c.id)}
            {idle}
            first={false}
            onfirst={(on) => void setFirst(c, on)}
            onremove={() => remove(c.id)}
          >
            {#snippet handle()}<span class="grip"><Icon name="grip" size={14} /></span>{/snippet}
          </MinerRow>
        {/snippet}
      </DragList>
    {:else}
      <p class="empty">Add channels here, or with “Add to the miner” when adding a tab.</p>
    {/if}
  {:else}
    <h2>Your follows <span class="count">{follows.length}</span></h2>
    {#each follows as c (c.id)}
      <MinerRow
        channel={c}
        info={info.get(c.id)}
        {idle}
        first={false}
        onfirst={(on) => void setFirst(c, on)}
        onskip={(on) => skip(c, on)}
      />
    {:else}
      <p class="empty">{miner.status?.running ? 'Loading your follows…' : 'Your follows show once the miner runs.'}</p>
    {/each}
    {#if s.skip.length}
      <h2>Skipped <span class="count">{s.skip.length}</span></h2>
      {#each s.skip as c (c.id)}
        <MinerRow channel={c} first={false} skipped onskip={(on) => skip(c, on)} />
      {/each}
    {/if}
  {/if}

  {#if miner.status?.drops.length}
    <h2>Drops in progress</h2>
    {#each miner.status.drops as d (d.name + d.game)}
      <div class="drop">
        <span class="drop-name">{d.name}{#if d.game}<span class="game"> · {d.game}</span>{/if}</span>
        <span class="bar"><span style:width="{Math.min(100, (d.minutes / Math.max(1, d.required)) * 100)}%"></span></span>
        <span class="mins">{d.minutes} / {d.required}m</span>
      </div>
    {/each}
  {/if}
</div>

<style>
  .channels {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-width: 760px;
  }
  .top {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin-bottom: 6px;
  }
  .note,
  .empty {
    color: var(--dim);
    font-size: 0.82rem;
  }
  .empty {
    margin: 6px 0;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 16px 0 6px;
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--muted);
  }
  .count {
    color: var(--dim);
    font-weight: 600;
  }
  .add {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 6px;
  }
  .picker {
    flex: 0 1 380px;
    min-width: 0;
  }
  .grip {
    display: inline-flex;
    color: var(--dim);
    cursor: grab;
  }
  .drop {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 160px auto;
    align-items: center;
    gap: 10px;
    padding: 4px 8px;
    font-size: 0.82rem;
  }
  .drop-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .game {
    color: var(--dim);
  }
  .bar {
    height: 6px;
    background: var(--surface);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .mins {
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
</style>
