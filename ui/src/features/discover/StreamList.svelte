<!-- Live streams a page at a time (1.7). Starts over when `key` changes. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
  import type { GameRef } from '../../ipc/bindings/GameRef';
  import type { Page } from '../../ipc/bindings/Page';
  import type { Nav } from '../browse/open.svelte';
  import StreamCard from '../browse/StreamCard.svelte';
  import { Paged } from './paged.svelte';
  import PagedEnd from './PagedEnd.svelte';

  interface Props {
    load: (after: string | null) => Promise<Page<ChannelCard>>;
    /** What the list shows; a new one loads it again. */
    key: string;
    nav: Nav;
    oncategory?: (game: GameRef) => void;
    ontag?: (tag: string) => void;
    showGame?: boolean;
  }

  let { load, key, nav, oncategory, ontag, showGame = true }: Props = $props();

  // svelte-ignore state_referenced_locally
  const list = new Paged<ChannelCard>(load);

  $effect(() => {
    void key;
    untrack(() => void list.reset(load));
  });
</script>

<div class="cards">
  {#each list.items as c (c.id)}
    <StreamCard card={c} {nav} {oncategory} {ontag} {showGame} />
  {/each}
</div>
<PagedEnd {list} empty="No one is live here right now." />

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 20px 14px;
  }
</style>
