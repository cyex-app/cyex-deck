<!-- V107-R36..R40 The toolbar on top of a stream manager list panel: search first, then a row
     of filters with the count on the right. Stays put while the list scrolls. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div class="smbar">{@render children()}</div>

<style>
  .smbar {
    position: sticky;
    top: 0;
    z-index: 1;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  /* One row of filters; the count or a button sits at its end. */
  .smbar :global(.filters) {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }
  .smbar :global(.count) {
    margin-left: auto;
    color: var(--dim);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
