<script lang="ts">
  import { tip } from './tooltip.svelte';

  const EDGE = 8;
  let width = $state(0);
  let viewport = $state(0);

  const left = $derived(Math.min(Math.max(tip.x - width / 2, EDGE), viewport - width - EDGE));
</script>

<svelte:window bind:innerWidth={viewport} />

{#if tip.visible}
  <div class="tip" role="tooltip" bind:clientWidth={width} style:left="{left}px" style:top="{tip.y}px">
    <span class="name">{tip.label}</span>
    {#if tip.hint}<span class="hint">{tip.hint}</span>{/if}
  </div>
{/if}

<style>
  .tip {
    position: fixed;
    z-index: var(--z-tooltip);
    max-width: 260px;
    padding: 6px 9px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--elevated-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 4px 14px var(--shadow);
    pointer-events: none;
    animation: in var(--fast) ease-out;
  }
  .name {
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
  }
  .hint {
    font-size: 0.76rem;
    color: var(--muted);
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
</style>
