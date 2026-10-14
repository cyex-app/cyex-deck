<!-- V105-14 A list whose rows drag into a new order, on the app's shared drag (tabs, tiles,
     panels): the row follows as a label, a line shows where it lands. Presses on a row's
     buttons and fields stay clicks. -->
<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';

  import { drag, rowDrops } from '../features/tabs/drag.svelte';
  import { moveKey } from '../lib/order';

  interface Props {
    /** Unique on screen: drops only land in the list they came from. */
    key: string;
    items: T[];
    id: (item: T) => string;
    /** The name that follows the pointer. */
    label: (item: T) => string;
    onmove: (items: T[]) => void;
    row: Snippet<[T]>;
  }

  let { key, items, id, label, onmove, row }: Props = $props();

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

  function down(e: PointerEvent, item: T) {
    if ((e.target as Element).closest('button, a, input, select, textarea, [contenteditable]')) return;
    drag.begin(e, { kind: 'row', list: key, id: id(item) }, label(item));
  }
</script>

<div class="list" role="list">
  {#each items as item (id(item))}
    {@const k = id(item)}
    <div
      class="row"
      class:before={line?.id === k && !line.after}
      class:after={line?.id === k && line.after}
      class:lifted={mine === k}
      role="listitem"
      data-row-of={key}
      data-row-id={k}
      onpointerdown={(e) => down(e, item)}
    >
      {@render row(item)}
    </div>
  {/each}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
  }
  .row {
    border-block: 2px solid transparent;
    cursor: grab;
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
