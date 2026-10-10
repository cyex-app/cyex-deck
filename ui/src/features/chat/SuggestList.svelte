<!-- C6 The list of suggestions above the message box. -->
<script lang="ts">
  import { settings } from '../../lib/settings.svelte';
  import type { Item } from './suggestions';
  import { emoteImg } from './urls';

  interface Props {
    items: Item[];
    index: number;
    onpick: (i: number) => void;
  }

  let { items, index, onpick }: Props = $props();

  const theme = $derived(settings.v.appearance.theme);
</script>

<div class="suggest" role="listbox">
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
        <img src={emoteImg(item.emote.provider, item.emote.id, theme, { animated: item.emote.animated }).src} alt="" />
      {:else if item.glyph}<span class="glyph">{item.glyph}</span>{/if}
      {item.label}
      {#if item.hint}<span class="hint">{item.hint}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .suggest {
    position: absolute;
    bottom: calc(100% - 2px);
    left: 8px;
    right: 8px;
    z-index: var(--z-popover);
    max-height: 260px;
    overflow-y: auto;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px var(--shadow);
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
