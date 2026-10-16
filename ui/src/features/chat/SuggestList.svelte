<!-- C6 The list of suggestions above the message box, as wide as it; below it or shorter
     when there's no room above (V102-03). -->
<script lang="ts">
  import { scrollBehavior } from '../../lib/motion';
  import { density } from '../../lib/density.svelte';
  import { floating } from '../../lib/place';
  import { settings } from '../../lib/settings.svelte';
  import type { Item } from './suggestions';
  import { emoteImg } from './urls';

  interface Props {
    items: Item[];
    index: number;
    onpick: (i: number) => void;
    /** The message box's row. */
    anchor: HTMLElement | undefined;
  }

  let { items, index, onpick, anchor }: Props = $props();

  const theme = $derived(settings.v.appearance.theme);
  let box = $state<HTMLElement>();

  // Arrowing past the visible rows keeps the active one in view.
  $effect(() => {
    void index;
    box?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest', behavior: scrollBehavior() });
  });
</script>

<div class="suggest" bind:this={box} role="listbox" use:floating={{ anchor, side: 'above', gap: 4, matchWidth: true }}>
  {#each items as item, i (item.value)}
    <button
      class="item"
      class:on={i === index}
      role="option"
      aria-selected={i === index}
      onmousedown={(e) => {
        e.preventDefault();
        onpick(i);
      }}
    >
      {#if item.emote}
        <img src={emoteImg(item.emote.provider, item.emote.id, theme, { animated: item.emote.animated, scale: density.scale }).src} alt="" />
      {:else if item.glyph}<span class="glyph">{item.glyph}</span>{/if}
      {item.label}
      {#if item.hint}<span class="hint">{item.hint}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .suggest {
    position: fixed;
    z-index: var(--z-popover);
    max-height: 260px;
    overflow-y: auto;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
  }
  .item img {
    height: 22px;
  }
  .glyph {
    width: 22px;
    font-size: 18px;
    text-align: center;
  }
  .hint {
    margin-left: auto;
    color: var(--dim);
    font-size: 0.75rem;
  }
  .item.on,
  .item:hover {
    background: var(--accent-dim);
  }
</style>
