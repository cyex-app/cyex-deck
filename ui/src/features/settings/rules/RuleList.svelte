<!-- A settings list of rules: one row each, an Add button, and a remove button per row. -->
<script lang="ts" generics="T extends Record<string, unknown>">
  import Button from '../../../components/Button.svelte';
  import EmptyState from '../../../components/EmptyState.svelte';
  import type { IconName } from '../../../components/icons';
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
    /** P1 Shown while the list is empty. */
    empty?: string;
    emptyIcon?: IconName;
    problem?: (item: T) => string | null;
    /** V103-13 Registry id: settings search jumps here. */
    setting?: string;
    /** Row layout, see RuleRow. */
    layout?: 'flow' | 'split';
    /** Option icons to make room for, see RuleRow. */
    slots?: number;
  }

  let { items, fields, make, onchange, addLabel = 'Add', empty, emptyIcon = 'list', problem, setting, layout, slots }: Props = $props();

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
    {#if empty}<EmptyState icon={emptyIcon} text={empty} />{/if}
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
  .add {
    padding-top: 8px;
  }
</style>
