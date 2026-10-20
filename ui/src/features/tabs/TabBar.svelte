<!-- Channel tabs (1.10) on any side of the window (U5), with groups, size (1.24),
     several selected at once (Ctrl or Shift click), dragging to reorder and the mouse
     wheel to step through them (V102-04). Across the
     top or bottom, tabs wrap onto more rows of one height (V101-38), up to the set number
     of rows and then a scrollbar, or all of them (F16); in the title bar the gaps between
     them still drag the window. "Only live tabs" leaves offline ones out (F17). -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { TabGroup } from '../../ipc/bindings/TabGroup';
  import { settings } from '../../lib/settings.svelte';
  import { openAdd, openEdit } from './dialog.svelte';
  import GroupChip from './GroupChip.svelte';
  import { GROUP_COLORS } from './groupColors';
  import { barMaxHeight } from './barLayout';
  import { stepTab, WheelSteps } from './cycle';
  import { barMenu, groupMenu, tabMenu } from './menus';
  import TabItem from './TabItem.svelte';
  import { tabs, tiles } from './tabs.svelte';
  import { scrollWithin } from '../../lib/scrollWithin';

  interface Props {
    vertical?: boolean;
    /** In the title bar. */
    chrome?: boolean;
    /** The tab on screen, `null` while a fixed view (home, settings) shows. */
    current: string | null;
    onopen: (id: string) => void;
    /** V106-26 Set by the window for a left bar; else the tab size's width. */
    width?: number;
  }

  let { vertical = false, chrome = false, current, onopen, width }: Props = $props();

  type Item = { kind: 'group'; group: TabGroup; count: number } | { kind: 'tab'; tab: Tab; color?: string };

  const DENSITY = {
    comfortable: { h: 32, pad: 12, font: '0.86rem', width: 200 },
    normal: { h: 28, pad: 10, font: '0.83rem', width: 176 },
    compact: { h: 23, pad: 7, font: '0.78rem', width: 148 },
  };
  const size = $derived(DENSITY[settings.v.appearance.tabDensity]);
  /** V106 a-7 A side bar's rows stack 2px apart: an even height keeps every row (and its divider)
   * on whole device pixels at 150 %; 23 + 2 put every other one between two. */
  const rowH = $derived(vertical ? size.h + (size.h % 2) : size.h);
  /** F16 A row is a tab and its gaps above and below (`--row-g`). In the title bar the
   * first row is as tall as the bar anyway. */
  const maxHeight = $derived.by(() => {
    if (vertical) return undefined;
    const a = settings.v.appearance;
    const rows = barMaxHeight(a.tabRows, a.tabMaxRows, size.h + 6);
    return chrome && a.tabRows === 'limit' ? `max(var(--row, 0px), ${rows})` : rows;
  });
  /** F17 */
  const hidden = $derived(tabs.hidden(current));
  /** The one tab Tab reaches: the shown one, else the first. */
  const stopId = $derived.by(() => {
    const ids = items.flatMap((i) => (i.kind === 'tab' ? [i.tab.id] : []));
    return current && ids.includes(current) ? current : (ids[0] ?? null);
  });

  const items = $derived.by(() => {
    const out: Item[] = [];
    let last: string | null = null;
    for (const tab of tabs.list) {
      if (hidden.has(tab.id)) continue;
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
    const tab = bar.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(current)}"]`);
    if (tab) scrollWithin(tab, 'nearest');
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
    // Under the tab, never over it (V102-03); from the keyboard too.
    const anchor = (e.currentTarget as HTMLElement | null)?.closest?.('[data-tab-id]');
    if (anchor) openMenu(anchor, tabMenu(tab, targets), 'start');
    else openMenu(e, tabMenu(tab, targets));
  }

  function close(tab: Tab) {
    if (selected.has(tab.id)) selected = new Set([...selected].filter((id) => id !== tab.id));
    tabs.close([tab.id]);
  }

  /** Right-click between tabs; tabs and group chips open their own menus. */
  function onbarmenu(e: MouseEvent) {
    if (e.defaultPrevented) return;
    e.preventDefault();
    openMenu(e, barMenu());
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
  style:--tab-h="{rowH}px"
  style:--tab-pad="{size.pad}px"
  style:--tab-font={size.font}
  style:--row-g={vertical ? undefined : '3px'}
  style:--bar-w="{width ?? size.width}px"
  style:max-height={maxHeight}
  oncontextmenu={onbarmenu}
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
        stop={item.tab.id === stopId}
        {vertical}
        groupColor={item.color}
        onselect={(e) => select(item.tab, e)}
        onedit={() => openEdit(item.tab.id, tiles(item.tab)[0].id)}
        onclose={() => close(item.tab)}
        onmenu={(e) => menuFor(item.tab, e)}
      />
    {/if}
  {/each}
  {#if hidden.size}
    <button
      class="offline"
      use:tooltip={{ label: 'Only live tabs', hint: 'Click to show offline tabs too.' }}
      onclick={() => void settings.patch('appearance', { tabOnlyLive: false })}
    >
      {hidden.size} offline
    </button>
  {/if}
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
    overflow-x: hidden;
    overflow-y: auto;
  }
  /* v1.07 t1 The app's own thin thumb. `scrollbar-width` would switch WebView2 to the native
     bar with arrow buttons and drop the ::-webkit-scrollbar look. */
  .bar::-webkit-scrollbar {
    width: 8px;
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
    --bar-pad: 6px;
    width: var(--bar-w);
    max-height: none;
    flex: none;
    padding: var(--bar-pad);
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
  /* F17 How many tabs "Only live tabs" leaves out; a click shows them again. */
  .offline {
    flex: none;
    align-self: center;
    height: calc(var(--tab-h) - 6px);
    padding: 0 8px;
    background: none;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    color: var(--dim);
    font-size: 0.75rem;
    font-weight: 600;
    white-space: nowrap;
  }
  .offline:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .vertical .offline {
    align-self: flex-start;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
