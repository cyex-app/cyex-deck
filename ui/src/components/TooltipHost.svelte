<script lang="ts">
  import { tip } from './tooltip.svelte';

  const EDGE = 8;
  let width = $state(0);
  let viewport = $state(0);

  const left = $derived(Math.min(Math.max(tip.x - width / 2, EDGE), viewport - width - EDGE));
</script>

<svelte:window bind:innerWidth={viewport} />

{#if tip.visible}
  <div class="tip" class:pic={!!tip.image} role="tooltip" bind:clientWidth={width} style:left="{left}px" style:top="{tip.y}px">
    {#if tip.image}<img src={tip.image} alt="" />{/if}
    <span class="name">{tip.label}</span>
    {#if tip.hint}<span class="hint">{tip.hint}</span>{/if}
    {#if tip.preview}
      {@const p = tip.preview}
      <div class="preview">
        {#if p.thumbnail && p.thumbWidth}
          <img class="thumb" src={p.thumbnail} alt="" style:max-width="{p.thumbWidth}px" referrerpolicy="no-referrer" />
        {/if}
        {#each p.lines as line, i (i)}
          {#if line.length}
            <p>{#each line as part, j (j)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}</p>
          {:else}<hr />{/if}
        {/each}
      </div>
    {/if}
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
    white-space: nowrap;
  }
  .hint {
    font-size: 0.76rem;
    color: var(--muted);
  }
  .tip:has(.preview) {
    max-width: 320px;
  }
  .preview {
    margin-top: 4px;
    font-size: 0.78rem;
    color: var(--muted);
  }
  .preview p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .preview b {
    color: var(--text);
  }
  .preview hr {
    margin: 4px 0;
    border: 0;
    border-top: 1px solid var(--border);
  }
  .thumb {
    display: block;
    width: 100%;
    max-height: 200px;
    object-fit: contain;
    margin-bottom: 4px;
    border-radius: var(--radius);
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
</style>
