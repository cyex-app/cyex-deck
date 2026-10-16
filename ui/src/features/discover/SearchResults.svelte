<!-- 1.7 Channels and categories matching what's typed. Live channels first. -->
<script lang="ts">
  import type { Category } from '../../ipc/bindings/Category';
  import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
  import { discoverIpc } from '../../ipc/browse';
  import ChannelRow from '../browse/ChannelRow.svelte';
  import type { Nav } from '../browse/open.svelte';
  import CategoryCard from './CategoryCard.svelte';
  import { discover } from './discover.svelte';
  import { errorText } from '../../lib/errors';

  let { text, nav }: { text: string; nav: Nav } = $props();

  let channels = $state.raw<ChannelCard[]>([]);
  let categories = $state.raw<Category[]>([]);
  let cursor = $state<string | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    const q = text;
    let alive = true;
    gen++;
    loading = true;
    error = null;
    discoverIpc
      .search(q)
      .then((r) => {
        if (!alive) return;
        channels = liveFirst(r.channels.items);
        cursor = r.channels.cursor;
        categories = r.categories;
      })
      .catch((e) => alive && (error = errorText(e)))
      .finally(() => alive && (loading = false));
    return () => (alive = false);
  });

  /** Keeps Twitch's order otherwise. */
  function liveFirst(list: ChannelCard[]): ChannelCard[] {
    return [...list.filter((c) => c.stream), ...list.filter((c) => !c.stream)];
  }

  /** Bumped per query: a "more" for an older one is dropped. */
  let gen = 0;

  async function more() {
    if (!cursor || loading) return;
    loading = true;
    const mine = gen;
    try {
      const page = await discoverIpc.searchMore(text, cursor);
      if (mine !== gen) return;
      const seen = new Set(channels.map((c) => c.id));
      channels = [...channels, ...page.items.filter((c) => !seen.has(c.id))];
      cursor = page.cursor;
    } catch (e) {
      if (mine !== gen) return;
      error = errorText(e);
      cursor = null;
    } finally {
      if (mine === gen) loading = false;
    }
  }
</script>

{#if error}
  <p class="note">{error}</p>
{:else if !loading && !channels.length && !categories.length}
  <p class="note">Nothing found for “{text}”.</p>
{/if}

{#if categories.length}
  <h2>Categories</h2>
  <div class="cats">
    {#each categories as c (c.id)}
      <CategoryCard category={c} onopen={(c) => discover.go({ kind: 'category', slug: c.slug, name: c.name })} />
    {/each}
  </div>
{/if}

{#if channels.length}
  <h2>Channels</h2>
  <div class="rows">
    {#each channels as c (c.id)}<ChannelRow card={c} {nav} />{/each}
  </div>
  {#if cursor}
    <button class="more" disabled={loading} onclick={more}>{loading ? 'Loading…' : 'More channels'}</button>
  {/if}
{:else if loading}
  <p class="note">Searching…</p>
{/if}

<style>
  h2 {
    margin: 4px 0 10px;
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--muted);
  }
  .cats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 14px 10px;
    margin-bottom: 20px;
  }
  .rows {
    display: flex;
    flex-direction: column;
  }
  .more {
    display: block;
    margin: 12px auto 0;
    padding: 6px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.85rem;
  }
  .more:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .note {
    margin: 0;
    padding: 40px 0;
    text-align: center;
    color: var(--dim);
  }
</style>
