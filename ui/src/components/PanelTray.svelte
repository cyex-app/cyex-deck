<!-- V103-06 The closed panels, each with its name and what it's for. Only while the layout
     is unlocked: a small chip under the panels, at their bottom right, that takes its own
     room and opens a menu of them. One drags back onto a side of any panel; a click puts it
     back where it sits by default. -->
<script lang="ts">
  import { drag } from '../features/tabs/drag.svelte';
  import type { TrayItem } from './panel';
  import Popover from './Popover.svelte';

  interface Props {
    /** The layout's key. */
    layout: string;
    items: TrayItem[];
    onrestore: (id: string) => void;
    /** V107-R29 In the dock's row, which has the padding. */
    inline?: boolean;
  }

  let { layout, items, onrestore, inline = false }: Props = $props();

  /** A press that turned into a drag isn't a click. */
  let dragged = false;
  let open = $state(false);

  function down(e: PointerEvent, item: TrayItem) {
    dragged = false;
    drag.begin(e, { kind: 'panel', layout, panel: item.id, tray: true }, item.title);
    const watch = () => {
      if (drag.item) {
        dragged = true;
        open = false;
      }
    };
    // L1 Also a cancelled pointer or a lost window: the move listener never stays behind.
    const end = (e: Event) => {
      removeEventListener('pointermove', watch);
      removeEventListener('pointerup', end);
      removeEventListener('pointercancel', end);
      removeEventListener('blur', end);
      // The click that may follow is the drag's own; the next press starts fresh.
      if (e.type === 'pointerup') setTimeout(() => (dragged = false));
      else dragged = false;
    };
    addEventListener('pointermove', watch);
    addEventListener('pointerup', end);
    addEventListener('pointercancel', end);
    addEventListener('blur', end);
  }

  function click(item: TrayItem) {
    if (!dragged) {
      onrestore(item.id);
      if (items.length <= 1) open = false;
    }
    dragged = false;
  }
</script>

<div class="tray" class:inline>
  <div class="wrap">
    <button class="chip" aria-haspopup="dialog" aria-expanded={open} onclick={() => (open = !open)}>
      Closed panels <span class="count">{items.length}</span>
    </button>
    <Popover bind:open side="above" label="Closed panels">
      <p class="tip">Drag one onto a panel, or click to put it back.</p>
      <ul>
        {#each items as item (item.id)}
          <li>
            <button
              class="item"
              onpointerdown={(e) => down(e, item)}
              onclick={() => click(item)}
            >
              <span class="title">{item.title}</span>
              <span class="hint">{item.hint}</span>
            </button>
          </li>
        {/each}
      </ul>
    </Popover>
  </div>
</div>

<style>
  .tray {
    flex: none;
    display: flex;
    justify-content: flex-end;
    padding: 4px 8px;
  }
  .tray.inline {
    padding: 0 0 0 6px;
  }
  .wrap {
    position: relative;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
    white-space: nowrap;
  }
  /* V107-My In the dock row it is as tall as the icons beside it. */
  .inline .chip {
    padding: 0 12px;
    font-size: var(--text-hint);
  }
  .chip:hover,
  .chip[aria-expanded='true'] {
    background: var(--surface-hover);
    border-color: var(--border-strong);
    color: var(--text);
  }
  .count {
    color: var(--dim);
  }
  .tip {
    margin: 0 0 4px;
    padding: 2px 8px;
    color: var(--dim);
    font-size: 0.72rem;
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 100%;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }
  li {
    min-width: 0;
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: 1px;
    width: 100%;
    padding: 4px 8px;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--text);
    text-align: left;
    cursor: grab;
  }
  .item:hover {
    background: var(--surface-hover);
  }
  .title,
  .hint {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
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
