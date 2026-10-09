<!-- Categories by viewers (1.7), with the ones you follow first (H4). -->
<script lang="ts">
  import { onMount } from 'svelte';

  import type { Category } from '../../ipc/bindings/Category';
  import { discoverIpc } from '../../ipc/browse';
  import { accounts } from '../../lib/accounts.svelte';
  import CategoryCard from './CategoryCard.svelte';
  import { discover } from './discover.svelte';
  import { Paged } from './paged.svelte';
  import PagedEnd from './PagedEnd.svelte';

  const list = new Paged<Category>((after) => discoverIpc.categories(after));
  let followed = $state.raw<Category[]>([]);

  const account = $derived(accounts.list.find((a) => a.isDefault && a.features.includes('follow'))?.userId);

  onMount(() => void list.reset());

  $effect(() => {
    const id = account;
    let alive = true;
    followed = [];
    if (id) {
      discoverIpc
        .followedCategories(id)
        .then((c) => alive && (followed = c.sort((a, b) => b.viewers - a.viewers)))
        .catch(() => {});
    }
    return () => (alive = false);
  });

  const open = (c: Category) => discover.go({ kind: 'category', slug: c.slug, name: c.name });
</script>

{#if followed.length}
  <h2>Yours</h2>
  <div class="grid">
    {#each followed as c (c.id)}<CategoryCard category={c} onopen={open} />{/each}
  </div>
  <h2>All</h2>
{/if}
<div class="grid">
  {#each list.items as c (c.id)}<CategoryCard category={c} onopen={open} />{/each}
</div>
<PagedEnd {list} empty="No categories right now." />

<style>
  h2 {
    margin: 4px 0 10px;
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--muted);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
    gap: 16px 12px;
    margin-bottom: 18px;
  }
</style>
