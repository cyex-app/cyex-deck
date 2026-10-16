<script lang="ts">
  import { toast } from '../lib/toast.svelte';

  const info = $derived(toast.items.filter((t) => t.kind !== 'error'));
  const errors = $derived(toast.items.filter((t) => t.kind === 'error'));
</script>

<!-- Errors are announced at once (alert), the rest politely (status). The regions stay in the
     page while empty, so screen readers catch what lands in them. Hover or focus holds one. -->
{#snippet stack(items: typeof toast.items, side: 'left' | 'right')}
  <div class="stack {side}" role={side === 'left' ? 'alert' : 'status'}>
    {#each items as t (t.id)}
      <button
        class="toast {t.kind}"
        onclick={() => toast.dismiss(t.id)}
        onmouseenter={() => toast.hold(t.id)}
        onmouseleave={() => toast.release(t.id)}
        onfocus={() => toast.hold(t.id)}
        onblur={() => toast.release(t.id)}
      >
        {t.text}{#if t.count > 1}<span class="count">×{t.count}</span>{/if}
      </button>
    {/each}
  </div>
{/snippet}

{@render stack(errors, 'left')}
{@render stack(info, 'right')}

<style>
  .stack {
    position: fixed;
    bottom: 14px;
    z-index: var(--z-toast);
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: min(380px, calc(100vw - 28px));
  }
  .left {
    left: 14px;
  }
  .right {
    right: 14px;
    align-items: flex-end;
  }
  .toast {
    padding: 10px 14px 10px 12px;
    text-align: left;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-left: 3px solid var(--accent);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    font-size: 0.85rem;
    animation: in var(--base) var(--ease);
  }
  .count {
    margin-left: 8px;
    padding: 0 5px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .toast.success {
    border-left-color: var(--green);
  }
  .toast.error {
    border-left-color: var(--red);
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
