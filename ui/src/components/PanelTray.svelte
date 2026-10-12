<!-- V103-06 The closed panels, each with its name and what it's for. Only while the layout
     is unlocked: a strip under the panels, at their bottom right, that takes its own room,
     so it never covers a chat box, a button, a scrollbar or a panel. One drags back onto a
     side of any panel; a click puts it back where it sits by default. -->
<script lang="ts">
  import { drag } from '../features/tabs/drag.svelte';
  import type { TrayItem } from './panel';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    /** The layout's key. */
    layout: string;
    items: TrayItem[];
    onrestore: (id: string) => void;
  }

  let { layout, items, onrestore }: Props = $props();

  /** A press that turned into a drag isn't a click. */
  let dragged = false;

  function down(e: PointerEvent, item: TrayItem) {
    dragged = false;
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

<div class="tray">
  <span class="head">Closed panels</span>
  <ul>
    {#each items as item (item.id)}
      <li>
        <button
          class="item"
          use:tooltip={'Drag it onto a panel, or click to put it back.'}
          onpointerdown={(e) => down(e, item)}
          onclick={() => click(item)}
        >
          <span class="title">{item.title}</span>
          <span class="hint">{item.hint}</span>
        </button>
      </li>
    {/each}
  </ul>
</div>

<style>
  .tray {
    flex: none;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    max-height: 132px;
    padding: 6px 8px;
    overflow: auto;
    background: var(--elevated);
    border-top: 1px solid var(--border);
  }
  .head {
    flex: none;
    padding: 6px 0;
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 700;
    white-space: nowrap;
  }
  ul {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
    min-width: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    min-width: 0;
    max-width: 220px;
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: 1px;
    width: 100%;
    padding: 4px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
    cursor: grab;
  }
  .item:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
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
