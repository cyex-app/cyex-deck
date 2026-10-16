<!-- V1051-03 What just happened to your look: saving, worn, an error, the combo edit bar.
     Sits right above the grid you picked from, and keeps its height so nothing moves. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { tooltip } from '../../components/tooltip.svelte';
  import Button from '../../components/Button.svelte';
  import { errorText, type CosmeticsStore } from './cosmetics.svelte';

  interface Props {
    store: CosmeticsStore;
    /** Takes the place of the status line. */
    bar?: Snippet;
  }

  let { store, bar }: Props = $props();

  const status = $derived(store.saveStatus);
</script>

<div class="status" aria-live="polite">
  {#if bar}
    {@render bar()}
  {:else if store.note}
    <span>{store.note.text}</span>
    {#if store.note.undo}<Button variant="ghost" small onclick={() => store.undo()}>Undo</Button>{/if}
  {:else if status === 'saving'}
    <span>Saving…</span>
  {:else if status === 'saved'}
    <!-- A pick, not a combo: say what changed. -->
    <span class="txt" use:tooltip={store.wearing}>{store.wearing}</span>
    {#if store.canUndo}<Button variant="ghost" small onclick={() => store.undo()}>Undo</Button>{/if}
  {:else if typeof status === 'object'}
    <span class="err">{errorText(status.error)}</span>
  {/if}
</div>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .txt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding-bottom: 2px;
    margin-bottom: -2px;
  }
  .status :global(button) {
    flex: none;
  }
  .err {
    color: var(--red);
  }
</style>
