<!-- A settings list of rules: one row each, an Add button, and a remove button per row. -->
<script lang="ts" generics="T extends Record<string, unknown>">
  import Button from '../../../components/Button.svelte';
  import type { Field } from './fields';
  import RuleRow from './RuleRow.svelte';

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
  }

  let { items, fields, make, onchange, addLabel = 'Add', empty, problem, setting }: Props = $props();
</script>

<div class="list" data-setting={setting}>
  {#each items as item, i (i)}
    <RuleRow
      {item}
      {fields}
      problem={problem?.(item)}
      onchange={(next) => onchange(items.map((x, j) => (j === i ? next : x)))}
      ondelete={() => onchange(items.filter((_, j) => j !== i))}
    />
  {:else}
    {#if empty}<p class="empty">{empty}</p>{/if}
  {/each}
  <div class="add"><Button small icon="plus" onclick={() => onchange([...items, make()])}>{addLabel}</Button></div>
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
