<!-- On and off chips that filter a list by kind: the activity feed (1.35) and the mod
     actions (V103-15). -->
<script lang="ts" module>
  export interface Chip {
    key: string;
    label: string;
    on: boolean;
  }
</script>

<script lang="ts">
  let { chips, onflip }: { chips: Chip[]; onflip: (key: string) => void } = $props();
</script>

<div class="chips" role="group" aria-label="Show">
  {#each chips as c (c.key)}
    <button class="chip" class:on={c.on} aria-pressed={c.on} onclick={() => onflip(c.key)}>{c.label}</button>
  {/each}
</div>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
  }
  .chip {
    padding: 1px 7px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--dim);
    font-size: 0.72rem;
    cursor: pointer;
  }
  .chip.on {
    background: var(--bg);
    border-color: var(--border-strong);
    color: var(--text);
  }
</style>
