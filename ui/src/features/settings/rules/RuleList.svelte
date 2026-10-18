<!-- A settings list of rules: one row each, an Add button, and a remove button per row.
     V106-10 With `reorder`, each row gets a grip: drag it, or Alt+↑/↓ on it. -->
<script lang="ts" generics="T extends Record<string, unknown>">
  import Button from '../../../components/Button.svelte';
  import DragList from '../../../components/DragList.svelte';
  import EmptyState from '../../../components/EmptyState.svelte';
  import type { IconName } from '../../../components/icons';
  import { applyEdit, type Field } from './fields';
  import { focusLast } from './focus';
  import RuleRow from './RuleRow.svelte';
  import { toast } from '../../../lib/toast.svelte';

  interface Props {
    items: T[];
    /** V106-10 Or per row, e.g. a regex flavor only some rows have. */
    fields: Field<T>[] | ((item: T, i: number) => Field<T>[]);
    /** A new, empty rule. */
    make: () => T;
    onchange: (items: T[]) => void;
    addLabel?: string;
    /** P1 Shown while the list is empty. */
    empty?: string;
    emptyIcon?: IconName;
    problem?: (item: T, i: number) => string | null;
    /** V106-10 A quiet line under the row, when there is no problem. */
    note?: (item: T, i: number) => string | null;
    /** See RuleRow. */
    mainMin?: string;
    /** CSS color an empty color field stands for, see RuleRow. */
    fallback?: string;
    /** V106-10 Grip, drag and Alt+↑/↓. `key` is unique on screen; `label` names the row. */
    reorder?: { key: string; label: (item: T, i: number) => string };
    /** V106-10 The Undo toast after a delete. */
    deleted?: string;
    /** V103-13 Registry id: settings search jumps here. */
    setting?: string;
    /** Row layout, see RuleRow. */
    layout?: 'flow' | 'split';
    /** Option icons to make room for, see RuleRow. */
    slots?: number;
  }

  let {
    items,
    fields,
    make,
    onchange,
    addLabel = 'Add',
    empty,
    emptyIcon = 'list',
    problem,
    note,
    mainMin,
    fallback,
    reorder,
    deleted,
    setting,
    layout,
    slots,
  }: Props = $props();

  const fieldsOf = (item: T, i: number) => (typeof fields === 'function' ? fields(item, i) : fields);

  let el = $state<HTMLDivElement>();

  /** b-16 "Move row 3", not "Move Row 3": a fallback name reads mid-sentence. */
  const grip = (name: string) => `Move ${/^(Row|Rule|Automation) \d+$/.test(name) ? name[0].toLowerCase() + name.slice(1) : name}`;

  /** Deletes at once; Undo puts the rule back where it was. */
  function remove(i: number) {
    const gone = items[i];
    onchange(items.filter((_, j) => j !== i));
    toast.success(deleted ?? 'Rule deleted.', {
      action: { label: 'Undo', run: () => onchange([...items.slice(0, i), gone, ...items.slice(i)]) },
    });
  }
</script>

{#snippet one(item: T, i: number, handle?: string)}
  <RuleRow
    {item}
    fields={fieldsOf(item, i)}
    {layout}
    {slots}
    {mainMin}
    {fallback}
    {handle}
    problem={problem?.(item, i)}
    note={note?.(item, i)}
    onchange={(next) => onchange(items.map((x, j) => (j === i ? applyEdit(x, item, next) : x)))}
    ondelete={() => remove(i)}
  />
{/snippet}

<div class="list" data-setting={setting} bind:this={el}>
  {#if reorder}
    {#if items.length}
      <DragList
        handle
        key={reorder.key}
        {items}
        id={(t) => String(items.indexOf(t))}
        label={(t) => reorder.label(t, items.indexOf(t))}
        onmove={onchange}
      >
        {#snippet row(item)}
          {@const i = items.indexOf(item)}
          {@render one(item, i, grip(reorder.label(item, i)))}
        {/snippet}
      </DragList>
    {:else if empty}
      <EmptyState align="start" icon={emptyIcon} text={empty} />
    {/if}
  {:else}
    {#each items as item, i (i)}
      {@render one(item, i)}
    {:else}
      {#if empty}<EmptyState align="start" icon={emptyIcon} text={empty} />{/if}
    {/each}
  {/if}
  <div class="add">
    <Button
      small
      icon="plus"
      onclick={() => {
        onchange([...items, make()]);
        void focusLast(el, '.rule');
      }}>{addLabel}</Button
    >
  </div>
</div>

<style>
  .list {
    margin-bottom: 8px;
  }
  .add {
    padding-top: 8px;
  }
</style>
