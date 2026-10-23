<!-- V1051-04 One combo: its live preview, its name and the worn mark. A click puts it on. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Combo } from '../../ipc/cosmetics';
  import { density } from '../../lib/density.svelte';
  import { focus } from '../../lib/focus.svelte';
  import { pairTitle, problemText } from './combos';
  import type { ComboStore } from './combos.svelte';
  import ComboName from './ComboName.svelte';
  import type { CosmeticsStore } from './cosmetics.svelte';
  import { badgePicture } from './pictures';

  interface Props {
    combo: Combo;
    combos: ComboStore;
    store: CosmeticsStore;
    name: string;
    worn: boolean;
    /** The one card in the tab order. */
    active: boolean;
  }

  let { combo, combos, store, name, worn, active }: Props = $props();

  const dpr = $derived(density.dpr());
  const badge = $derived(combo.badge ? store.badgeById.get(combo.badge.id) : undefined);
  const owned = $derived(!!combo.paint && store.paintById.has(combo.paint.id));
  const problem = $derived(problemText(combos.problemsOf(combo)));
  const renaming = $derived(combos.renaming === combo.id);
  let err = $state<string | null>(null);
  let el = $state<HTMLElement>();

  function menu(e: MouseEvent) {
    e.stopPropagation();
    openMenuAt(e, [
      { label: 'Rename', icon: 'pencil', hint: 'F2', run: () => (combos.renaming = combo.id) },
      { label: 'Edit', icon: 'palette', run: () => combos.edit(combo) },
      { label: 'Duplicate', icon: 'copy', hint: 'Ctrl+D', disabled: combos.full, run: () => void combos.duplicate(combo) },
      'separator',
      { label: 'Delete', icon: 'trash', danger: true, hint: 'Del', run: () => void combos.remove(combo) },
    ]);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  bind:this={el}
  class="card"
  class:on={worn}
  class:naming={renaming}
  role="option"
  aria-selected={worn}
  tabindex={active ? 0 : -1}
  data-id={combo.id}
  use:tooltip={pairTitle(combo)}
  onclick={() => !renaming && void combos.apply(combo)}
>
  {#if renaming && err}
    <span class="err">{err}</span>
  {:else}
    <span class="live">
      {#if badge}
        <img src={badgePicture(badge.images, badge.stills, 18, dpr, focus.idle)} width="18" height="18" alt="" decoding="async" />
      {/if}
      <span class="back"><span class="who {owned ? `painted paint-${combo.paint?.id}` : ''}">{name}</span></span>
    </span>
  {/if}
  {#if renaming}
    <ComboName
      value={combo.name}
      onsubmit={async (n) => {
        const e = await combos.rename(combo, n);
        if (!e) combos.renaming = null;
        return e;
      }}
      oncancel={(k) => {
        combos.renaming = null;
        err = null;
        if (k) void tick().then(() => el?.focus());
      }}
      onerror={(m) => (err = m)}
    />
  {:else}
    <span class="nm">{combo.name}</span>
  {/if}
  {#if problem && !renaming}
    <span class="warn" use:tooltip={problem}><Icon name="alert" size={12} /></span>
  {/if}
  {#if !renaming}
    <span class="more"><IconButton icon="more" label="Combo menu" size={14} onclick={menu} /></span>
  {/if}
</div>

<style>
  .card {
    position: relative;
    box-sizing: border-box;
    display: flex;
    flex: none;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    width: 176px;
    height: 56px;
    padding: 6px 8px;
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    scroll-snap-align: start;
    transition:
      border-color var(--fast),
      transform var(--fast) ease-out;
  }
  .card:hover {
    border-color: var(--border-strong);
  }
  .card.on {
    padding: 5px 7px;
    border: 2px solid var(--accent-line);
  }
  .card.naming {
    padding-block: 3px;
  }
  .card.naming.on {
    padding-block: 2px;
  }
  .card:active {
    transform: scale(0.97);
  }
  .card:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .live {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .live img {
    flex: none;
    object-fit: contain;
  }
  /* Light: pale paints sit on the same soft grey as in the grid (ItemGrid .back). */
  .back {
    display: flex;
    min-width: 0;
    padding: 0 6px;
    background: var(--paint-back);
    border-radius: var(--radius);
  }
  .who {
    padding-block: 0.25em;
    margin-block: -0.25em;
    overflow: hidden;
    font-size: 14px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nm {
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* Room for descenders under the clip. */
    padding-bottom: 2px;
    margin-bottom: -2px;
  }
  /* Wraps inside the card, two lines at most (like the Save card). */
  .err {
    flex: none;
    min-height: 24px;
    max-height: 24px;
    overflow: hidden;
    color: var(--red);
    font-size: 11px;
    line-height: 12px;
    overflow-wrap: anywhere;
  }
  .warn {
    position: absolute;
    top: 5px;
    right: 6px;
    display: flex;
    color: var(--red);
  }
  .more {
    position: absolute;
    top: 2px;
    right: 2px;
    display: none;
  }
  .card:hover .more,
  .card:focus-within .more {
    display: block;
  }
  /* V1051-04 The mark moves left of the menu button, so its tip can still be hovered. */
  .card:hover .warn,
  .card:focus-within .warn {
    right: 30px;
  }
  .more :global(.icon-btn) {
    width: 24px;
    height: 24px;
  }
  @media (prefers-reduced-motion: reduce) {
    .card {
      transition: none;
    }
    .card:active {
      transform: none;
    }
  }
</style>
