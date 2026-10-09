<!-- Channel tabs (1.10) on any side of the window (U5), with groups, size (1.24),
     several selected at once (Ctrl or Shift click) and dragging to reorder. -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { TabGroup } from '../../ipc/bindings/TabGroup';
  import { settings } from '../../lib/settings.svelte';
  import { openAdd, openEdit } from './dialog.svelte';
  import GroupChip from './GroupChip.svelte';
  import { GROUP_COLORS } from './groupColors';
  import { groupMenu, tabMenu } from './menus';
  import TabItem from './TabItem.svelte';
  import { tabs, tiles } from './tabs.svelte';

  interface Props {
    vertical?: boolean;
    /** The tab on screen, `null` while a fixed view (home, settings) shows. */
    current: string | null;
    onopen: (id: string) => void;
  }

  let { vertical = false, current, onopen }: Props = $props();

  type Item = { kind: 'group'; group: TabGroup; count: number } | { kind: 'tab'; tab: Tab; color?: string };

  const DENSITY = {
    comfortable: { h: 32, pad: 12, font: '0.86rem', width: 200 },
    normal: { h: 28, pad: 10, font: '0.83rem', width: 176 },
    compact: { h: 23, pad: 7, font: '0.78rem', width: 148 },
  };
  const size = $derived(DENSITY[settings.v.appearance.tabDensity]);

  const items = $derived.by(() => {
    const out: Item[] = [];
    let last: string | null = null;
    for (const tab of tabs.list) {
      const group = tabs.group(tab.group);
      if (group && group.id !== last) {
        out.push({ kind: 'group', group, count: tabs.list.filter((t) => t.group === group.id).length });
      }
      last = group?.id ?? null;
      if (group?.collapsed && tab.id !== current) continue;
      out.push({ kind: 'tab', tab, color: group ? GROUP_COLORS[group.color].css : undefined });
    }
    return out;
  });

  let selected = $state<Set<string>>(new Set());
  let anchor: string | null = null;
  let bar = $state<HTMLElement>();

  // Keep the shown tab in view when the bar scrolls.
  $effect(() => {
    if (!current || !bar) return;
    bar.querySelector(`[data-tab-id="${CSS.escape(current)}"]`)?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  });

  function select(tab: Tab, e: MouseEvent) {
    const visible = items.flatMap((i) => (i.kind === 'tab' ? [i.tab.id] : []));
    if (e.ctrlKey) {
      const next = new Set(selected.size ? selected : current ? [current] : []);
      if (next.has(tab.id)) next.delete(tab.id);
      else next.add(tab.id);
      selected = next;
      anchor = tab.id;
      return;
    }
    if (e.shiftKey && (anchor ?? current)) {
      const a = visible.indexOf((anchor ?? current)!);
      const b = visible.indexOf(tab.id);
      if (a >= 0 && b >= 0) {
        selected = new Set(visible.slice(Math.min(a, b), Math.max(a, b) + 1));
        return;
      }
    }
    selected = new Set();
    anchor = tab.id;
    onopen(tab.id);
  }

  function menuFor(tab: Tab, e: MouseEvent) {
    const many = selected.has(tab.id) && selected.size > 1;
    const targets = many ? tabs.list.filter((t) => selected.has(t.id)) : [tab];
    openMenu(e, tabMenu(tab, targets));
  }

  function close(tab: Tab) {
    if (selected.has(tab.id)) selected = new Set([...selected].filter((id) => id !== tab.id));
    tabs.close([tab.id]);
  }

  function onwheel(e: WheelEvent) {
    if (!vertical && bar && e.deltaY) bar.scrollLeft += e.deltaY;
  }
</script>

<div
  class="bar"
  class:vertical
  data-tab-bar
  role="tablist"
  aria-orientation={vertical ? 'vertical' : 'horizontal'}
  tabindex="-1"
  bind:this={bar}
  {onwheel}
  style:--tab-h="{size.h}px"
  style:--tab-pad="{size.pad}px"
  style:--tab-font={size.font}
  style:--bar-w="{size.width}px"
>
  {#each items as item (item.kind === 'tab' ? item.tab.id : `g:${item.group.id}`)}
    {#if item.kind === 'group'}
      <GroupChip
        group={item.group}
        count={item.count}
        {vertical}
        onmenu={(e) => openMenu(e, groupMenu(item.group))}
      />
    {:else}
      <TabItem
        tab={item.tab}
        active={item.tab.id === current}
        selected={selected.has(item.tab.id)}
        {vertical}
        groupColor={item.color}
        onselect={(e) => select(item.tab, e)}
        onedit={() => openEdit(item.tab.id, tiles(item.tab)[0].id)}
        onclose={() => close(item.tab)}
        onmenu={(e) => menuFor(item.tab, e)}
      />
    {/if}
  {/each}
  <div class="add">
    <IconButton icon="plus" label="Add a tab" hint="Any channel, by name, user id or link." size={15} onclick={() => openAdd()} />
  </div>
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }
  .bar::-webkit-scrollbar {
    display: none;
  }
  .vertical {
    flex-direction: column;
    align-items: stretch;
    width: var(--bar-w);
    flex: none;
    padding: 6px;
    overflow-x: hidden;
    overflow-y: auto;
    scrollbar-width: thin;
  }
  .add {
    flex: none;
  }
  .vertical .add {
    align-self: flex-start;
  }
</style>
