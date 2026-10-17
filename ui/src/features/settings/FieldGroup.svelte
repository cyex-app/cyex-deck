<!-- V106-14 Form fields drawn as one block. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    legend: string;
    hint?: string;
    setting?: string;
    children: Snippet;
  }

  let { legend, hint, setting, children }: Props = $props();
</script>

<fieldset class="group" data-setting={setting}>
  <legend class="label">{legend}</legend>
  {#if hint}<p class="hint">{hint}</p>{/if}
  <div class="fields">{@render children()}</div>
</fieldset>

<style>
  .group {
    margin: 14px 0 6px;
    padding: 10px 12px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  legend {
    padding: 0 4px;
    margin-left: -4px;
  }
  .hint {
    margin: 0 0 10px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>
