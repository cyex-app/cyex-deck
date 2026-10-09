<!-- One settings line: label and optional hint on the left, control on the right. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    hint?: string;
    /** Puts the control under the label instead of beside it. */
    stacked?: boolean;
    children: Snippet;
  }

  let { label, hint, stacked = false, children }: Props = $props();
</script>

<div class="row" class:stacked>
  <div class="text">
    <span class="title">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 11px 0;
    border-bottom: 1px solid var(--border);
  }
  .row.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .title {
    font-weight: 600;
  }
  .hint {
    font-size: 0.8rem;
    color: var(--dim);
  }
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .stacked .control {
    justify-content: flex-start;
  }
</style>
