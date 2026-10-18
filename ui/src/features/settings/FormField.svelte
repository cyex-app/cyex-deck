<!-- V106-14 One labelled input in a FieldGroup: name, "Required" tag, hint or problem line. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    hint?: string;
    required?: boolean;
    problem?: string | null;
    setting?: string;
    /** The input. */
    children: Snippet;
  }

  let { label, hint, required = false, problem = null, setting, children }: Props = $props();
</script>

<label class="ff" data-setting={setting}>
  <span class="top"><span class="name">{label}</span>{#if required}<span class="req">Required</span>{/if}</span>
  {@render children()}
  {#if problem}<span class="problem" aria-live="polite">{problem}</span>{:else if hint}<span class="hint">{hint}</span>{/if}
</label>

<style>
  .ff {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .name {
    font-weight: 600;
    font-size: 0.85rem;
  }
  .req {
    margin-left: 6px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--muted);
    font-size: 0.68rem;
    font-weight: 600;
  }
  .hint {
    color: var(--dim);
    font-size: 0.78rem;
  }
  .problem {
    color: var(--red);
    font-size: 0.78rem;
  }
  .ff :global(.field) {
    width: 100%;
  }
</style>
