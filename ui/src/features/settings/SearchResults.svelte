<!-- V103-13 Settings search results: each says where it lives (group › page). Arrow keys
     in the search box move the pick, Enter or a click jumps to the setting. -->
<script lang="ts">
  import type { Hit } from './search';

  interface Props {
    hits: Hit[];
    query: string;
    /** The result Enter picks. */
    active: number;
    onpick: (hit: Hit) => void;
  }

  let { hits, query, active, onpick }: Props = $props();

  let list = $state<HTMLElement>();

  // Keep the picked one in view while arrowing through a long list.
  $effect(() => {
    void active;
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  });
</script>

{#if hits.length}
  <ul class="hits" bind:this={list} aria-label="Search results">
    {#each hits as hit, i (hit.entry.id)}
      <li>
        <button class="hit" class:on={i === active} aria-current={i === active} onclick={() => onpick(hit)}>
          <span class="top">
            <span class="name">{hit.entry.label}</span>
            <span class="where">{hit.page.group} › {hit.page.label}</span>
          </span>
          {#if hit.entry.description}<span class="hint">{hit.entry.description}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
{:else}
  <p class="none">No settings match “{query.trim()}”.</p>
{/if}

<style>
  .hits {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .hit {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 8px 10px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
  }
  .hit:hover {
    background: var(--surface-hover);
  }
  .hit.on {
    background: var(--accent-dim);
  }
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    column-gap: 10px;
    min-width: 0;
  }
  .name {
    font-weight: 600;
  }
  .where {
    margin-left: auto;
    flex: none;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .hint {
    overflow: hidden;
    color: var(--dim);
    font-size: 0.8rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .none {
    margin: 4px 0;
    color: var(--dim);
  }
</style>
