<script lang="ts">
  import { toast } from '../lib/toast.svelte';

  const info = $derived(toast.items.filter((t) => t.kind !== 'error'));
  const errors = $derived(toast.items.filter((t) => t.kind === 'error'));
</script>

{#snippet stack(items: typeof toast.items, side: 'left' | 'right')}
  <div class="stack {side}" aria-live="polite">
    {#each items as t (t.id)}
      <button class="toast {t.kind}" onclick={() => toast.dismiss(t.id)}>{t.text}</button>
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
    box-shadow: 0 8px 24px var(--shadow);
    font-size: 0.85rem;
    animation: in 0.2s var(--ease);
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
