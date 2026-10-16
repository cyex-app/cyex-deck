<!-- A settings list of rules: one row each, an Add button, and a remove button per row. -->
<script lang="ts" generics="T extends Record<string, unknown>">
  import Button from '../../../components/Button.svelte';
  import type { Field } from './fields';
  import { focusLast } from './focus';
  import RuleRow from './RuleRow.svelte';
  import { toast } from '../../../lib/toast.svelte';

  interface Props {
    items: T[];
    fields: Field<T>[];
    /** A new, empty rule. */
    make: () => T;
    onchange: (items: T[]) => void;
    addLabel?: string;
    empty?: string;
    problem?: (item: T) => string | null;
    /** V103-13 Registry id: settings search jumps here. */
    setting?: string;
    /** Row layout, see RuleRow. */
    layout?: 'flow' | 'split';
    /** Option icons to make room for, see RuleRow. */
    slots?: number;
  }

  let { items, fields, make, onchange, addLabel = 'Add', empty, problem, setting, layout, slots }: Props = $props();

  let el = $state<HTMLDivElement>();

  /** Deletes at once; Undo puts the rule back where it was. */
  function remove(i: number) {
    const gone = items[i];
    onchange(items.filter((_, j) => j !== i));
    toast.success('Rule deleted.', {
      action: { label: 'Undo', run: () => onchange([...items.slice(0, i), gone, ...items.slice(i)]) },
    });
  }
</script>

<div class="list" data-setting={setting} bind:this={el}>
  {#each items as item, i (i)}
    <RuleRow
      {item}
      {fields}
      {layout}
      {slots}
      problem={problem?.(item)}
      onchange={(next) => onchange(items.map((x, j) => (j === i ? next : x)))}
      ondelete={() => remove(i)}
    />
  {:else}
    {#if empty}<p class="empty">{empty}</p>{/if}
  {/each}
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
  .empty {
    margin: 6px 0;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .add {
    padding-top: 8px;
  }
</style>
