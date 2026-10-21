<!-- V107-R37, R39 Above a list you can tick: Pick all (a dash when some are picked), how many
     are picked, and what to do with them on the right. The actions drop to their own line
     when the panel is narrow, so the count is never cut. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import Checkbox from '../../components/Checkbox.svelte';
  import { formatNumber } from '../../lib/format';

  interface Props {
    picked: number;
    total: number;
    onall: (on: boolean) => void;
    children?: Snippet;
  }

  let { picked, total, onall, children }: Props = $props();

  const all = $derived(total > 0 && picked === total);
</script>

<div class="pickbar">
  <span class="all">
    <Checkbox label={picked ? `${formatNumber(picked)} picked` : 'Pick all'} checked={all} indeterminate={picked > 0 && !all} onchange={onall} />
  </span>
  {#if children}<span class="acts">{@render children()}</span>{/if}
</div>

<style>
  .pickbar {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 8px;
    min-height: 36px;
    padding: 4px 10px;
    border-bottom: 1px solid var(--border);
  }
  .all {
    flex: none;
    color: var(--muted);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .acts {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 4px;
    margin-left: auto;
  }
</style>
