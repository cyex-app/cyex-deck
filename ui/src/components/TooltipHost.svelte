<script lang="ts">
  import { floating } from '../lib/place';
  import LinkTipCard from './LinkTipCard.svelte';
  import { tip } from './tooltip.svelte';
</script>

{#if tip.visible}
  <!-- V101-04 Under its element, or above it when there's no room below (or a side it asks for, ct-9). -->
  <div class="tip" class:pic={!!tip.image} role="tooltip" use:floating={{ anchor: tip.anchor, side: tip.side, align: 'center' }}>
    {#if tip.image}<img src={tip.image} alt="" />{/if}
    <span class="name {tip.paint}" class:painted={!!tip.paint}>{tip.label}</span>
    {#if tip.hint}<span class="hint">{tip.hint}</span>{/if}
    {#if tip.card}<LinkTipCard c={tip.card} />{/if}
  </div>
{/if}

<style>
  .tip {
    position: fixed;
    z-index: var(--z-tooltip);
    /* V102-03 Its own width wherever it lands, so a long name wraps the same everywhere. */
    width: max-content;
    max-width: 260px;
    padding: 6px 9px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--elevated-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    pointer-events: none;
    animation: in var(--fast) ease-out;
  }
  .pic {
    align-items: center;
    text-align: center;
  }
  .pic img {
    max-width: 224px;
    height: 112px;
    object-fit: contain;
    margin-bottom: 4px;
  }
  .name {
    font-size: 0.8rem;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .hint {
    font-size: 0.76rem;
    color: var(--muted);
  }
  /* V108-C25 Room for a link's card. */
  .tip:has(:global(.tip-card)) {
    max-width: 320px;
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
</style>
