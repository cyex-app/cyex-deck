<!-- The name that follows the pointer while a tab or tile is dragged. -->
<script lang="ts">
  import { drag } from './drag.svelte';
</script>

{#if drag.item}
  <div class="ghost" style:left="{drag.x + 12}px" style:top="{drag.y + 10}px">{drag.label}</div>
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
    box-shadow: 0 6px 18px var(--shadow);
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
