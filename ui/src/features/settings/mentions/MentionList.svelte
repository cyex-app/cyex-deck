<!-- V104-01 A list of mentions with an Add button. Patterns are checked in Rust (one
     engine for every flavor), so a broken regex shows why right under it. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import type { MentionCheck } from '../../../ipc/bindings/MentionCheck';
  import type { MentionRule } from '../../../ipc/bindings/MentionRule';
  import { mentionsIpc } from '../../../ipc/mentions';
  import { focusLast } from '../rules/focus';
  import MentionRow from './MentionRow.svelte';
  import { newMention } from './status';

  interface Props {
    items: MentionRule[];
    onchange: (items: MentionRule[]) => void;
    empty: string;
    /** V103-13 Registry id: settings search jumps here. */
    setting: string;
  }

  let { items, onchange, empty, setting }: Props = $props();

  let el = $state<HTMLDivElement>();
  let checks = $state.raw<MentionCheck[]>([]);
  $effect(() => {
    const list = items;
    let live = true;
    mentionsIpc
      .check(list)
      .then((c) => live && (checks = c))
      .catch(() => {});
    return () => (live = false);
  });
</script>

<div class="list" data-setting={setting} bind:this={el}>
  {#each items as item, i (i)}
    <MentionRow
      {item}
      check={checks[i]}
      onchange={(next) => onchange(items.map((x, j) => (j === i ? (next as MentionRule) : x)))}
      ondelete={() => onchange(items.filter((_, j) => j !== i))}
    />
  {:else}
    <p class="empty">{empty}</p>
  {/each}
  <div class="add">
    <Button
      small
      icon="plus"
      onclick={() => {
        onchange([...items, newMention()]);
        void focusLast(el, '.mention');
      }}>Add</Button
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
