<!-- V109-B05 A slot's pop-up under its slot, in the look of the 7TV paint and badge pop-up:
     a titled header with ×, then the body. Escape or a click outside closes it and the focus
     goes back to the slot; a click on the slot itself is left to the slot (it closes it). -->
<script lang="ts">
  import { onMount, type Snippet } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { layer } from '../../lib/escape';
  import { floating } from '../../lib/place';

  interface Props {
    title: string;
    anchor: HTMLElement;
    onclose: () => void;
    children: Snippet;
  }

  let { title, anchor, onclose, children }: Props = $props();

  let box = $state<HTMLElement>();

  onMount(() => {
    const first = box?.querySelector<HTMLElement>('input, [role="listbox"]');
    (first ?? box)?.focus();
    const off = layer('popover', onclose);
    const down = (e: MouseEvent) => {
      const t = e.target as Element | null;
      if (box && t && !box.contains(t) && !anchor.contains(t) && !t.closest('[data-shared-menu]')) onclose();
    };
    addEventListener('mousedown', down, true);
    return () => {
      off();
      removeEventListener('mousedown', down, true);
      // Back to the slot, unless the click went somewhere else that took the focus.
      if (!document.activeElement || document.activeElement === document.body || box?.contains(document.activeElement)) anchor.focus();
    };
  });
</script>

<div class="pop float-in" bind:this={box} tabindex="-1" role="dialog" aria-label={title} use:floating={{ anchor, side: 'below', align: 'start' }}>
  <header>
    <span>{title}</span>
    <button class="x" aria-label="Close" onclick={onclose}><Icon name="x" size={14} /></button>
  </header>
  <div class="body">{@render children()}</div>
</div>

<style>
  .pop {
    position: fixed;
    z-index: var(--z-popover);
    display: flex;
    flex-direction: column;
    /* Three badge cards across (3 × 72 + gaps and padding). */
    width: 300px;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    outline: none;
    box-shadow: var(--shadow-pop);
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
    border-radius: var(--radius);
    color: var(--dim);
  }
  .x:hover {
    color: var(--text);
  }
  .x:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    padding: 10px;
  }
</style>
