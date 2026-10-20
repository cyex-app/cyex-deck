<!-- V103-13 Settings search results: each says where it lives (group › page). Arrow keys
     in the search box move the pick, Enter or a click jumps to the setting. -->
<script lang="ts">
  import { scrollWithin } from '../../lib/scrollWithin';
  import { pieces, whereOf, type Hit } from './search';

  interface Props {
    hits: Hit[];
    query: string;
    /** The result Enter picks. */
    active: number;
    onpick: (hit: Hit) => void;
    /** S4 The pointer over a result makes it the picked one. */
    onhover?: (index: number) => void;
  }

  let { hits, query, active, onpick, onhover }: Props = $props();

  let list = $state<HTMLElement>();

  // Keep the picked one in view while arrowing through a long list.
  $effect(() => {
    void active;
    const on = list?.querySelector<HTMLElement>('.on');
    if (on) scrollWithin(on, 'nearest');
  });
</script>

{#if hits.length}
  <ul class="hits" bind:this={list} aria-label="Search results">
    {#each hits as hit, i (hit.entry.id)}
      <li>
        <button
          class="hit"
          class:on={i === active}
          aria-current={i === active}
          onclick={() => onpick(hit)}
          onpointermove={() => i !== active && onhover?.(i)}
        >
          <span class="top">
            <span class="name">{#each pieces(hit.entry.label, query) as p, k (k)}{#if p.hit}<mark>{p.text}</mark>{:else}{p.text}{/if}{/each}</span>
            <span class="where">{whereOf(hit.page)}</span>
          </span>
          {#if hit.entry.description}<span class="hint">{hit.entry.description}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
{:else}
  <p class="none">No settings match “{query.trim().length > 60 ? `${query.trim().slice(0, 60)}…` : query.trim()}”.</p>
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
  /* The pointer moves the pick (S4), so only one row is ever lit. */
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
  mark {
    padding: 0;
    background: none;
    color: var(--accent-text);
  }
  .where {
    margin-left: auto;
    flex: none;
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .hint {
    overflow: hidden;
    color: var(--dim);
    font-size: var(--text-hint);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .none {
    margin: 4px 0;
    color: var(--dim);
    /* t5 A pasted wall of text wraps instead of scrolling the page sideways. */
    overflow-wrap: anywhere;
  }
</style>
