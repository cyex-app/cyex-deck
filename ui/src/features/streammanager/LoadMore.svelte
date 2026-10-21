<!-- V107-R37..R40 "Load more" under a stream manager list, with an optional note beside it. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';

  let { busy, note = '', onclick }: { busy: boolean; note?: string; onclick: () => void } = $props();
</script>

<div class="more">
  {#if note}<span>{note}</span>{/if}
  <Button small disabled={busy} {onclick}>{busy ? 'Loading…' : 'Load more'}</Button>
</div>

<style>
  .more {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px 8px 12px;
    color: var(--muted);
    font-size: 0.78rem;
    text-align: center;
  }
</style>
