<!-- The name that follows the pointer while a tab or tile is dragged. V107-R46 Near the
     window's right or bottom edge it flips to the pointer's other side, so a panel dragged
     out of the dock's bottom row keeps its whole name. -->
<script lang="ts">
  import { ghostAt } from './ghost';
  import { drag } from './drag.svelte';

  let width = $state(0);
  let height = $state(0);
  let view = $state({ width: innerWidth, height: innerHeight });
  const at = $derived(ghostAt({ x: drag.x, y: drag.y }, { width, height }, view));
</script>

<svelte:window onresize={() => (view = { width: innerWidth, height: innerHeight })} />

{#if drag.item}
  <div class="ghost" style:left="{at.left}px" style:top="{at.top}px" bind:offsetWidth={width} bind:offsetHeight={height}>
    {drag.label}
  </div>
{/if}

<style>
  .ghost {
    position: fixed;
    z-index: var(--z-tooltip);
    max-width: 220px;
    padding: 4px 10px;
    overflow: hidden;
    background: var(--elevated-2);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    font-size: 0.82rem;
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
    pointer-events: none;
  }
  :global(body.dragging),
  :global(body.dragging *) {
    cursor: grabbing !important;
  }
</style>
