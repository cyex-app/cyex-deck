<script lang="ts">
  import { floating } from '../lib/place';
  import LinkTipCard from './LinkTipCard.svelte';
  import { hideTip, overTipNow, tip } from './tooltip.svelte';
</script>

{#if tip.visible}
  <!-- V101-04 Under its element, or above it when there's no room below (or a side it asks for, ct-9). -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="tip"
    class:pic={!!tip.image}
    class:acts={tip.actions.length > 0}
    role="tooltip"
    use:floating={{ anchor: tip.anchor, side: tip.side, align: 'center' }}
    onmouseenter={() => tip.actions.length && overTipNow(true)}
    onmouseleave={() => tip.actions.length && overTipNow(false)}
  >
    {#if tip.image}<img src={tip.image} alt="" />{/if}
    <span class="name {tip.paint}" class:painted={!!tip.paint}>{tip.label}</span>
    {#if tip.hint}<span class="hint">{tip.hint}</span>{/if}
    {#if tip.card}<LinkTipCard c={tip.card} />{/if}
    {#if tip.actions.length}
      <!-- V110-U59 -->
      <div class="acts-row">
        {#each tip.actions as a (a.label)}
          <button
            type="button"
            onclick={() => {
              a.run();
              hideTip();
            }}>{a.label}</button
          >
        {/each}
      </div>
    {/if}
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
  .tip.acts {
    pointer-events: auto;
  }
  .acts-row {
    display: flex;
    gap: 4px;
    margin-top: 6px;
  }
  .acts-row button {
    padding: 2px 8px;
    background: var(--surface-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.74rem;
    cursor: pointer;
  }
  .acts-row button:hover {
    border-color: var(--border-strong);
  }
  .acts-row button:active {
    opacity: 0.6;
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
