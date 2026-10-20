<!-- V106-14 One labelled input in a FieldGroup: name, "Required" tag, hint, problem line.
     b-11 The problem goes under the hint, so the hint is there when it's needed. -->
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
  {#if hint}<span class="hint">{hint}</span>{/if}
  <span class="problem" aria-live="polite">{problem ?? ''}</span>
</label>

<style>
  .ff {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .name {
    font-weight: 600;
    font-size: var(--text-sub);
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
    font-size: var(--text-hint);
  }
  .problem {
    color: var(--red);
    font-size: var(--text-hint);
  }
  /* rb-5 V107-R27 Every field keeps the line (not only required ones), so "Fill this in." moves nothing
     and all fields in a group have the same height and gaps. */
  .problem {
    min-height: 1lh;
  }
  .ff :global(.field) {
    width: 100%;
  }
</style>
