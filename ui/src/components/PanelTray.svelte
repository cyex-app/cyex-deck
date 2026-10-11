<!-- V103-06 The closed panels, in the layout's bottom right corner: each with its name and
     what it's for. Unlocked, one drags back onto a side of any panel; locked or not, a
     click puts it back where it sits by default. Locked, it starts folded down to one small
     button, so it doesn't cover the panels under it. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import { drag } from '../features/tabs/drag.svelte';
  import Icon from './Icon.svelte';
  import type { TrayItem } from './panel';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    /** The layout's key. */
    layout: string;
    items: TrayItem[];
    unlocked: boolean;
    onrestore: (id: string) => void;
  }

  let { layout, items, unlocked, onrestore }: Props = $props();

  /** Open while the layout is unlocked, folded to one button while it's locked; a click
   * on it changes that until the lock does. */
  let open = $state(untrack(() => unlocked));
  $effect(() => {
    open = unlocked;
  });
  /** A press that turned into a drag isn't a click. */
  let dragged = false;

  function down(e: PointerEvent, item: TrayItem) {
    dragged = false;
    if (!unlocked) return;
    drag.begin(e, { kind: 'panel', layout, panel: item.id, tray: true }, item.title);
    const watch = () => {
      if (drag.item) dragged = true;
    };
    addEventListener('pointermove', watch);
    addEventListener(
      'pointerup',
      () => {
        removeEventListener('pointermove', watch);
        // The click that may follow is the drag's own; the next press starts fresh.
        setTimeout(() => (dragged = false));
      },
      { once: true },
    );
  }

  function click(item: TrayItem) {
    if (!dragged) onrestore(item.id);
    dragged = false;
  }
</script>

<div class="tray" class:open>
  <button class="head" aria-expanded={open} onclick={() => (open = !open)}>
    <span>Closed panels</span>
    <span class="count">{items.length}</span>
    <Icon name={open ? 'chevronDown' : 'chevronUp'} size={13} />
  </button>
  {#if open}
    <ul>
      {#each items as item (item.id)}
        <li>
          <button
            class="item"
            class:grab={unlocked}
            use:tooltip={unlocked ? 'Drag it onto a panel, or click to put it back.' : 'Click to put it back.'}
            onpointerdown={(e) => down(e, item)}
            onclick={() => click(item)}
          >
            <span class="title">{item.title}</span>
            <span class="hint">{item.hint}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .tray {
    position: absolute;
    right: 10px;
    bottom: 10px;
    z-index: 9;
    display: flex;
    flex-direction: column;
    width: 220px;
    max-height: calc(100% - 20px);
    background: var(--elevated-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 6px 18px var(--shadow);
  }
  .tray:not(.open) {
    width: auto;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    background: none;
    border: 0;
    color: var(--text);
    font-size: 0.75rem;
    font-weight: 700;
    cursor: pointer;
  }
  .head span:first-child {
    flex: 1;
    text-align: left;
  }
  .count {
    padding: 0 5px;
    background: var(--surface);
    border-radius: var(--radius);
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  ul {
    margin: 0;
    padding: 0 4px 4px;
    overflow: auto;
    list-style: none;
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: 1px;
    width: 100%;
    padding: 5px 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .item.grab {
    cursor: grab;
  }
  .item:hover {
    background: var(--surface);
  }
  .title {
    font-size: 0.8rem;
    font-weight: 600;
  }
  .hint {
    color: var(--muted);
    font-size: 0.72rem;
  }
</style>
