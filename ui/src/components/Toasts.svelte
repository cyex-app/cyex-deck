<script lang="ts">
  import { toast } from '../lib/toast.svelte';

</script>

<!-- One stack, top centre under the title bar. The region is polite (status); an error inside it is announced at
     once (alert). The region stays in the page while empty, so screen readers catch what lands
     in it. Hover or focus holds one. -->
<div class="stack" role="status">
  {#each toast.items as t (t.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="toast {t.kind}"
      role={t.kind === 'error' ? 'alert' : undefined}
      onmouseenter={() => toast.hold(t.id)}
      onmouseleave={() => toast.release(t.id)}
      onfocusin={() => toast.hold(t.id)}
      onfocusout={() => toast.release(t.id)}
    >
      <button class="text" onclick={() => toast.dismiss(t.id)}>
        {t.text}{#if t.count > 1}<span class="count">×{t.count}</span>{/if}
      </button>
      {#if t.action}<button class="act" onclick={() => toast.act(t.id)}>{t.action.label}</button>{/if}
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    top: calc(var(--chrome-h) + 10px);
    left: 50%;
    transform: translateX(-50%);
    width: max-content;
    align-items: center;
    pointer-events: none;
    z-index: var(--z-toast);
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: min(420px, calc(100vw - 28px));
  }
  .toast {
    display: flex;
    align-items: center;
    /* Translucent, so the page title under it still reads. */
    background: color-mix(in srgb, var(--elevated) 78%, transparent);
    backdrop-filter: blur(14px) saturate(1.2);
    border: 1px solid var(--border-strong);
    border-left: 3px solid var(--accent-line);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    font-size: 0.85rem;
    pointer-events: auto;
    animation: in var(--base) var(--ease);
  }
  .text {
    flex: 1;
    /* t5 A long name with no spaces (a layout, a file) wraps inside the toast. */
    min-width: 0;
    overflow-wrap: anywhere;
    padding: 10px 14px 10px 12px;
    background: none;
    border: 0;
    text-align: left;
  }
  .act {
    flex: none;
    align-self: stretch;
    padding: 0 14px;
    background: none;
    border: 0;
    border-left: 1px solid var(--border);
    color: var(--accent-text);
    font-weight: 600;
  }
  .act:hover {
    background: var(--surface-hover);
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
      transform: translateY(-8px);
    }
  }
  /* V107-H13a Less transparency asked for: the one glass surface turns solid. */
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) {
    .toast {
      background: var(--elevated);
      backdrop-filter: none;
    }
  }
</style>
