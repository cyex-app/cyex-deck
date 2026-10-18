<!-- V105-14 A list whose rows drag into a new order, on the app's shared drag (tabs, tiles,
     panels): the row follows as a label, a line shows where it lands. Presses on a row's
     buttons and fields stay clicks. -->
<script lang="ts" generics="T">
  import { tick, type Snippet } from 'svelte';

  import { drag, rowDrops } from '../features/tabs/drag.svelte';
  import { moveKey, nudge, nudgeDir } from '../lib/order';

  interface Props {
    /** Unique on screen: drops only land in the list they came from. */
    key: string;
    items: T[];
    id: (item: T) => string;
    /** The name that follows the pointer. */
    label: (item: T) => string;
    onmove: (items: T[]) => void;
    row: Snippet<[T]>;
    /** V106-10 Drag and Alt+↑/↓ only from the row's [data-drag-handle] (rows full of fields). */
    handle?: boolean;
  }

  let { key, items, id, label, onmove, row, handle = false }: Props = $props();

  $effect(() => {
    const k = key;
    const f = (from: string, to: string, after: boolean) => {
      const next = moveKey(items, id, from, to, after);
      if (next) onmove(next);
    };
    rowDrops.set(k, f);
    return () => {
      if (rowDrops.get(k) === f) rowDrops.delete(k);
    };
  });

  const mine = $derived(drag.item?.kind === 'row' && drag.item.list === key ? drag.item.id : null);
  const line = $derived.by(() => {
    const t = drag.target;
    return mine !== null && t?.kind === 'row' && t.list === key && t.id !== mine ? t : null;
  });

  let said = $state('');
  let box = $state<HTMLElement>();

  /** Alt+ArrowUp / Alt+ArrowDown on a focused row (or its handle) moves it, like a drag does. */
  async function nudgeRow(e: KeyboardEvent, item: T) {
    const dir = nudgeDir(e);
    if (dir === null) return;
    if (handle ? !(e.target as Element).closest('[data-drag-handle]') : e.target !== e.currentTarget) return;
    e.preventDefault();
    const row = e.currentTarget as HTMLElement;
    const moved = nudge(items, id, id(item), dir);
    if (!moved) return;
    onmove(moved.items);
    said = `Moved to ${moved.place} of ${items.length}`;
    await tick();
    if (handle) {
      // V106-10 By position: ids may be indexes, so an id lookup would find the wrong row.
      box?.querySelectorAll<HTMLElement>(':scope > .row')[moved.place - 1]?.querySelector<HTMLElement>('[data-drag-handle]')?.focus();
    } else if (!row.isConnected) box?.querySelector<HTMLElement>(`[data-row-id="${CSS.escape(id(item))}"]`)?.focus();
    else row.focus();
  }

  function down(e: PointerEvent, item: T) {
    if (handle) {
      if (!(e.target as Element).closest('[data-drag-handle]')) return;
    } else if ((e.target as Element).closest('button, a, input, select, textarea, [contenteditable]')) return;
    drag.begin(e, { kind: 'row', list: key, id: id(item) }, label(item));
  }
</script>

<div class="list" role="list" bind:this={box}>
  {#each items as item (id(item))}
    {@const k = id(item)}
    <!-- A row is a stop for Alt+Up / Alt+Down, the keyboard way to drag it. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="row"
      class:before={line?.id === k && !line.after}
      class:after={line?.id === k && line.after}
      class:lifted={mine === k}
      class:handled={handle}
      role="listitem"
      data-row-of={key}
      data-row-id={k}
      tabindex={handle ? undefined : 0}
      onkeydown={(e) => nudgeRow(e, item)}
      onpointerdown={(e) => down(e, item)}
    >
      {@render row(item)}
    </div>
  {/each}
</div>
<span class="sr-only" aria-live="polite">{said}</span>

<style>
  .list {
    display: flex;
    flex-direction: column;
  }
  .row {
    border-block: 2px solid transparent;
    cursor: grab;
  }
  /* V106-10 Only the handle drags; the rest of the row is fields. */
  .row.handled {
    cursor: auto;
  }
  .row:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
    border-radius: var(--radius);
  }
  .row.before {
    border-top-color: var(--accent);
  }
  .row.after {
    border-bottom-color: var(--accent);
  }
  .row.lifted {
    opacity: 0.5;
  }
</style>
