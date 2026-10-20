<!-- V1051-04 Saved looks in one row: click one to put it on. The first card saves what you wear. -->
<script lang="ts">
  import { tick } from 'svelte';
  import { slide } from 'svelte/transition';

  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { formatNumber } from '../../lib/format';
  import { reduced, scrollBehavior } from '../../lib/motion';
  import ComboCard from './ComboCard.svelte';
  import { MAX_COMBOS, type ComboStore } from './combos.svelte';
  import ComboName from './ComboName.svelte';
  import type { CosmeticsStore } from './cosmetics.svelte';
  import { scrollWithin } from '../../lib/scrollWithin';

  interface Props {
    combos: ComboStore;
    store: CosmeticsStore;
    name: string;
  }

  let { combos, store, name }: Props = $props();

  let row = $state<HTMLElement>();
  let saveBtn = $state<HTMLButtonElement>();
  let saveErr = $state<string | null>(null);
  let activeId = $state<string | null>(null);
  const list = $derived(combos.list);
  const tabbable = $derived(list.find((c) => c.id === activeId)?.id ?? combos.worn?.id ?? list[0]?.id ?? null);

  const cards = () => [...(row?.querySelectorAll<HTMLElement>('[role="option"]') ?? [])];

  function go(el: HTMLElement | undefined) {
    if (!el) return;
    activeId = el.dataset.id ?? null;
    void tick().then(() => {
      el.focus();
      scrollWithin(el, 'nearest', scrollBehavior(), 'x');
    });
  }

  // A new card (saved, duplicated, restored) takes focus once it is on the list.
  $effect(() => {
    const id = combos.focusId;
    if (!id || !list.some((c) => c.id === id)) return;
    combos.focusId = null;
    activeId = id;
    void tick().then(() => go(cards().find((e) => e.dataset.id === id)));
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLButtonElement) return;
    const all = cards();
    const at = all.findIndex((el) => el === document.activeElement);
    const here = list.find((c) => c.id === all[at]?.dataset.id);
    if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') go(all[Math.max(0, Math.min(all.length - 1, at + (e.key === 'ArrowRight' ? 1 : -1)))]);
    else if (e.key === 'Home') go(all[0]);
    else if (e.key === 'End') go(all[all.length - 1]);
    else if (!here) return;
    else if (e.key === 'Enter' || e.key === ' ') void combos.apply(here);
    else if (e.key === 'F2') combos.renaming = here.id;
    else if (e.key === 'Delete') void combos.remove(here);
    else if (e.ctrlKey && e.key.toLowerCase() === 'd') void combos.duplicate(here);
    else if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) all[at]?.querySelector('button')?.click();
    else return;
    e.preventDefault();
    e.stopPropagation();
  }

  function wheel(e: WheelEvent) {
    if (!row || e.deltaX !== 0 || row.scrollWidth <= row.clientWidth) return;
    e.preventDefault();
    row.scrollLeft += e.deltaY;
  }

  function stopNaming(key = false) {
    combos.naming = false;
    saveErr = null;
    if (key) void tick().then(() => saveBtn?.focus());
  }
</script>

<section class="combos" aria-label="Combos">
  <h3>Combos{list.length ? ` · ${formatNumber(list.length)}` : ''}</h3>
  <div class="row" bind:this={row} onwheel={wheel}>
    {#if combos.naming}
      <div class="save naming">
        <ComboName
          onsubmit={async (n) => {
            const e = await combos.save(n);
            if (!e) stopNaming();
            return e;
          }}
          oncancel={(k) => stopNaming(k)}
          onerror={(m) => (saveErr = m)}
        />
        <span class="err">{saveErr ?? ''}</span>
      </div>
    {:else}
      <button
        bind:this={saveBtn}
        class="save"
        disabled={combos.full || !!combos.editing}
        onclick={() => (combos.naming = true)}
        use:tooltip={combos.full ? `You have ${MAX_COMBOS} combos. Delete one first.` : 'Save what you wear now'}
      >
        <Icon name="plus" size={14} />
        <span>Save as combo</span>
      </button>
    {/if}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="list" role="listbox" aria-label="Your combos" tabindex="-1" {onkeydown}>
      {#each list as c (c.id)}
        <div class="slot" role="none" out:slide={{ axis: 'x', duration: reduced() ? 0 : 160 }}>
          <ComboCard combo={c} {combos} {store} {name} worn={combos.worn?.id === c.id} active={tabbable === c.id} />
        </div>
      {/each}
    </div>
    <!-- V1051-04 In the strip, so the page doesn't jump when the first combo lands. -->
    {#if !list.length}
      <p class="empty">Save what you wear now to put it on again with one click.</p>
    {/if}
  </div>
</section>

<style>
  .combos {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  h3 {
    margin: 0;
    color: var(--dim);
    font-size: 0.75rem;
    font-weight: 600;
  }
  .row {
    display: flex;
    gap: 8px;
    padding: 2px;
    overflow-x: auto;
    /* Scrolls by wheel and arrow keys; no bar under the cards. */
    scrollbar-width: none;
    scroll-snap-type: x proximity;
  }
  .list {
    display: flex;
    gap: 8px;
    outline: none;
  }
  .slot {
    flex: none;
    display: flex;
    /* Cards scrolled out of sight stop painting, so their animated paints rest. */
    content-visibility: auto;
    contain-intrinsic-size: 176px 56px;
  }
  .save {
    flex: none;
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: center;
    gap: 6px;
    background: none;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    box-sizing: border-box;
    width: 176px;
    height: 56px;
    scroll-snap-align: start;
    color: var(--muted);
    font-size: 12px;
    transition:
      border-color var(--fast),
      color var(--fast),
      transform var(--fast) ease-out;
  }
  .save:hover:not(:disabled) {
    border-color: var(--text);
    color: var(--text);
  }
  .save:active:not(:disabled) {
    transform: scale(0.97);
  }
  .save:disabled {
    opacity: 0.4;
  }
  .save.naming {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    justify-content: center;
    gap: 2px;
    padding: 3px 8px;
    border-style: solid;
  }
  /* Two lines fit the longest message (the card is 176 px wide); the height is fixed so
     the card doesn't jump. */
  .err {
    min-height: 24px;
    max-height: 24px;
    overflow: hidden;
    color: var(--red);
    font-size: 11px;
    line-height: 12px;
    overflow-wrap: anywhere;
  }
  /* V107-R5b No combos yet and no room beside the save card (a narrow chat): the hint wraps
     under it instead of being cut off by the row's scroll. */
  .row:has(.empty) {
    flex-wrap: wrap;
  }
  .empty {
    flex: 1 1 160px;
    align-self: center;
    max-width: 260px;
    margin: 0;
    color: var(--muted);
    font-size: 0.8rem;
  }
  @media (prefers-reduced-motion: reduce) {
    .save {
      transition: none;
    }
    .save:active:not(:disabled) {
      transform: none;
    }
  }
</style>
