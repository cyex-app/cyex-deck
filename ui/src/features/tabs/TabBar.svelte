<!-- Channel tabs (1.10) on any side of the window (U5), with groups, size (1.24),
     several selected at once (Ctrl or Shift click), dragging to reorder and the mouse
     wheel to step through them (V102-04). Across the
     top or bottom, tabs wrap onto more rows of one height instead of scrolling (V101-38);
     in the title bar the gaps between them still drag the window. -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { TabGroup } from '../../ipc/bindings/TabGroup';
  import { settings } from '../../lib/settings.svelte';
  import { openAdd, openEdit } from './dialog.svelte';
  import GroupChip from './GroupChip.svelte';
  import { GROUP_COLORS } from './groupColors';
  import { stepTab, WheelSteps } from './cycle';
  import { groupMenu, tabMenu } from './menus';
  import TabItem from './TabItem.svelte';
  import { tabs, tiles } from './tabs.svelte';

  interface Props {
    vertical?: boolean;
    /** In the title bar. */
    chrome?: boolean;
    /** The tab on screen, `null` while a fixed view (home, settings) shows. */
    current: string | null;
    onopen: (id: string) => void;
  }

  let { vertical = false, chrome = false, current, onopen }: Props = $props();

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

  // V102-04 The wheel over the bar steps through tabs: down is the next one, up the one
  // before, stopping at the ends. It doesn't scroll the bar; the shown tab stays in view.
  const wheel = new WheelSteps();
  $effect(() => {
    const el = bar;
    if (!el) return;
    const onwheel = (e: WheelEvent) => {
      if (e.ctrlKey) return;
      e.preventDefault();
      const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
      const by = wheel.step(delta, e.deltaMode !== WheelEvent.DOM_DELTA_PIXEL, e.timeStamp);
      const next = stepTab(tabs.shownIds(current), current, by, false);
      if (next) onopen(next);
    };
    el.addEventListener('wheel', onwheel, { passive: false });
    return () => el.removeEventListener('wheel', onwheel);
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

</script>

<div
  class="bar"
  class:vertical
  class:chrome
  data-tab-bar
  data-tauri-drag-region={chrome ? '' : undefined}
  role="tablist"
  aria-orientation={vertical ? 'vertical' : 'horizontal'}
  tabindex="-1"
  bind:this={bar}
  style:--tab-h="{size.h}px"
  style:--tab-pad="{size.pad}px"
  style:--tab-font={size.font}
  style:--row-g={vertical ? undefined : '3px'}
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
        onedit={() => !tiles(item.tab)[0].feed && openEdit(item.tab.id, tiles(item.tab)[0].id)}
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
  /* Rows stretch to fill a taller parent, so one row in the title bar is as tall as the
     bar and reaches its top edge (V101-19). No side padding: the divider before a row's
     first tab falls outside and is clipped. */
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
    align-content: stretch;
    column-gap: 2px;
    min-width: 0;
    max-height: 40vh;
    overflow-x: hidden;
    overflow-y: auto;
    scrollbar-width: thin;
  }
  .chrome {
    flex: 0 1 auto;
    align-self: stretch;
  }
  .vertical {
    flex-direction: column;
    flex-wrap: nowrap;
    align-content: normal;
    gap: 2px;
    width: var(--bar-w);
    max-height: none;
    flex: none;
    padding: 6px;
  }
  .add {
    display: flex;
    flex: none;
    align-self: center;
  }
  /* As tall as a tab, so the row it ends up in is no taller than the others. */
  .add :global(.icon-btn) {
    height: var(--tab-h);
  }
  .vertical .add {
    align-self: flex-start;
  }
</style>
