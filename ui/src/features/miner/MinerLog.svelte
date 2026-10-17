<!-- W4 The miner log, newest first (V106-27): points, chests, raids, bets, drops, channels
     and errors, with a filter per kind and Clear. Kept in memory, not saved. -->
<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';

  import Button from '../../components/Button.svelte';
  import FeedList from '../../components/FeedList.svelte';
  import FilterChips from '../../components/FilterChips.svelte';
  import { minerIpc } from '../../ipc/miner';
  import { CHIPS, visible } from './minerLog';
  import { miner } from './miner.svelte';

  // Per window run only.
  const off = new SvelteSet<string>();

  const chips = $derived(CHIPS.map((c) => ({ ...c, on: !off.has(c.key) })));
  const rows = $derived(visible(miner.log, off).map((l) => ({ ...l, key: String(l.id), ts: l.t })));

  function flip(key: string): void {
    if (!off.delete(key)) off.add(key);
  }
</script>

<div class="log">
  <div class="top">
    <FilterChips {chips} onflip={flip} />
    <Button small variant="ghost" disabled={miner.log.length === 0} onclick={() => void minerIpc.clearLog()}>Clear</Button>
  </div>
  <FeedList
    {rows}
    empty={miner.log.length === 0
      ? 'Nothing yet. Chests, raids, bets and drops show up here as they happen.'
      : 'Nothing here with these filters.'}
  >
    {#snippet row(l)}
      <span class:err={l.kind === 'error'}>
        {#if l.channel}<b>{l.channel}</b>{/if}
        {l.text}
      </span>
    {/snippet}
  </FeedList>
</div>

<style>
  .log {
    max-width: 760px;
    font-size: 0.85rem;
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .err {
    color: var(--red);
  }
</style>
