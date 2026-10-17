<!-- U5 A tab group's label. Click folds its tabs away; dropping a tab on it adds the tab.
     A folded group lights up like its brightest tab (U9). -->
<script lang="ts">
  import { tooltip } from '../../components/tooltip.svelte';
  import type { TabGroup } from '../../ipc/bindings/TabGroup';
  import { drag } from './drag.svelte';
  import { GROUP_COLORS } from './groupColors';
  import { tabs } from './tabs.svelte';

  interface Props {
    group: TabGroup;
    count: number;
    vertical: boolean;
    onmenu: (e: MouseEvent) => void;
  }

  let { group, count, vertical, onmenu }: Props = $props();

  const unread = $derived.by(() => {
    if (!group.collapsed) return null;
    let state: 'message' | 'mention' | null = null;
    for (const t of tabs.list) {
      if (t.group !== group.id) continue;
      const u = tabs.unreadState(t);
      if (u === 'mention') return u;
      state ??= u;
    }
    return state;
  });

  const over = $derived(drag.item?.kind === 'tab' && drag.target?.kind === 'group' && drag.target.id === group.id);

  function toggle() {
    void tabs.apply({ op: 'editGroup', group: group.id, name: null, color: null, collapsed: !group.collapsed });
  }
</script>

<button
  class="chip"
  class:vertical
  class:collapsed={group.collapsed}
  class:over
  class:message={unread === 'message'}
  class:mention={unread === 'mention'}
  data-group-id={group.id}
  style:--group={GROUP_COLORS[group.color].css}
  onclick={toggle}
  oncontextmenu={(e) => {
    e.preventDefault();
    onmenu(e);
  }}
  use:tooltip={{
    label: group.collapsed ? `Show ${count} tabs` : 'Fold this group',
    hint: 'Right-click to rename, recolor or ungroup.',
  }}
>
  <span class="dot"></span>
  {#if group.name}<span class="name">{group.name}</span>{/if}
  {#if group.collapsed}<span class="count">{count}</span>{/if}
</button>

<style>
  .chip {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    max-width: 160px;
    height: calc(var(--tab-h) - 6px);
    padding: 0 8px;
    background: none;
    border: 1px solid var(--group);
    border-radius: var(--radius);
    font-size: var(--tab-font);
    font-weight: 700;
    color: var(--text);
  }
  .chip:not(.vertical) {
    align-self: center;
  }
  .chip.vertical {
    min-width: 0;
    max-width: none;
    margin-top: 4px;
  }
  .chip:hover,
  .over {
    background: var(--surface-hover);
  }
  .over {
    box-shadow: 0 0 0 2px var(--accent-border);
  }
  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--group);
  }
  .name {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    flex: none;
    color: var(--muted);
    font-weight: 600;
  }
  .message .count,
  .mention .count {
    color: var(--text);
  }
  .mention {
    box-shadow: inset 0 2px var(--accent);
  }
</style>
