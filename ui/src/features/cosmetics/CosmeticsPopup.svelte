<!-- V1051-03 /cosmetics: the paint and badge menu as a small pop-up over a chat, just
     above the message box. Escape (with the search empty) or a click outside closes it. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import CosmeticsPanel from './CosmeticsPanel.svelte';

  let { account, onclose }: { account: string; onclose: () => void } = $props();

  let box = $state<HTMLElement>();

  onMount(() => {
    box?.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
    // A click anywhere else closes it; the consent dialog and menus live outside the box.
    const down = (e: MouseEvent) => {
      const t = e.target as Element | null;
      if (box && t && !box.contains(t) && !t.closest('.backdrop, .menu')) onclose();
    };
    addEventListener('mousedown', down, true);
    return () => removeEventListener('mousedown', down, true);
  });
</script>

<div class="pop" bind:this={box} role="dialog" aria-label="7TV paint and badge">
  <header>
    <span>7TV paint and badge</span>
    <button class="x" aria-label="Close" onclick={onclose}><Icon name="x" size={14} /></button>
  </header>
  <div class="body"><CosmeticsPanel {account} compact {onclose} /></div>
</div>

<style>
  .pop {
    position: absolute;
    right: 8px;
    bottom: 8px;
    z-index: 5;
    display: flex;
    flex-direction: column;
    width: min(380px, calc(100% - 16px));
    height: min(520px, calc(100% - 16px));
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 6px 24px rgb(0 0 0 / 0.35);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 6px 6px 12px;
    border-bottom: 1px solid var(--border);
    font-weight: 700;
    font-size: 0.85rem;
  }
  .x {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .x:hover {
    color: var(--text);
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 10px;
  }
</style>
