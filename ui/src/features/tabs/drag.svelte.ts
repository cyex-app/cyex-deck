// Dragging tabs, tiles and panels (U3, U5, V102-23) with pointer events: reorder tabs,
// tile one tab's channel into another, move tiles around or out into their own tab, and
// with a layout unlocked put any panel on a side of another (V103-05, V103-06): in a tab,
// the mod view or the stream manager, from its place or from the closed-panels corner.

import type { Zone } from '../../ipc/bindings/Zone';
import type { Edge } from '../../lib/panels';
import { tabs, tiles } from './tabs.svelte';
import { edgeAt, groupAt, rowSlot, zoneAt } from './zones';

/** V103-05 A panel of the layout `layout` (a `PanelLayout`'s key). `tile` when it's a tab's,
 * so dropping it on another tile moves the tile (U3); `tray` when it comes from the
 * closed-panels corner. */
export interface PanelItem {
  kind: 'panel';
  layout: string;
  panel: string;
  tile?: string;
  tray?: boolean;
}

export type DragItem = { kind: 'tab'; id: string } | { kind: 'tile'; id: string } | PanelItem;

export type DropTarget =
  /** Next to a tab in the bar, or into it for a tile. */
  | { kind: 'tab'; id: string; after: boolean }
  | { kind: 'group'; id: string }
  /** The bar's empty end. */
  | { kind: 'bar' }
  | { kind: 'tile'; id: string; zone: Zone }
  /** V103-05 On an edge of another panel of the same layout, or of the whole layout. */
  | { kind: 'panel'; layout: string; panel: string | null; edge: Edge };

/** V103-05 What each layout on screen does with a panel dropped on it, by its key. */
export const panelDrops = new Map<string, (panel: string, target: string | null, edge: Edge) => void>();

/** Pixels the pointer moves before a press turns into a drag. */
const THRESHOLD = 5;

class Drag {
  item = $state<DragItem | null>(null);
  label = $state('');
  x = $state(0);
  y = $state(0);
  target = $state<DropTarget | null>(null);

  /** Call on pointerdown. Dragging starts once the pointer moves a little. */
  begin(e: PointerEvent, item: DragItem, label: string): void {
    if (e.button !== 0) return;
    const start = { x: e.clientX, y: e.clientY };
    const move = (ev: PointerEvent) => {
      if (!this.item) {
        if (Math.hypot(ev.clientX - start.x, ev.clientY - start.y) < THRESHOLD) return;
        this.item = item;
        this.label = label;
        document.body.classList.add('dragging');
      }
      this.x = ev.clientX;
      this.y = ev.clientY;
      this.target = hit(ev.clientX, ev.clientY, item);
    };
    const end = (drop: boolean) => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);
      removeEventListener('keydown', key);
      removeEventListener('blur', cancel);
      document.body.classList.remove('dragging');
      if (drop && this.item && this.target) apply(this.item, this.target);
      this.item = null;
      this.target = null;
    };
    const up = () => end(true);
    const cancel = () => end(false);
    const key = (ev: KeyboardEvent) => ev.key === 'Escape' && cancel();
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
    addEventListener('keydown', key);
    addEventListener('blur', cancel);
  }
}

export const drag = new Drag();

function hit(x: number, y: number, item: DragItem): DropTarget | null {
  const el = document.elementFromPoint(x, y);
  if (!el) return null;
  const kind = item.kind;
  if (item.kind === 'panel') {
    // V103-05 Onto another panel of its layout, or the layout itself; a tab's panel onto
    // another tile moves the whole tile (U3).
    const key = CSS.escape(item.layout);
    const panel = el.closest<HTMLElement>(`[data-panel-of="${key}"]`);
    if (panel) {
      if (panel.dataset.panel === item.panel) return null;
      return { kind: 'panel', layout: item.layout, panel: panel.dataset.panel!, edge: edgeAt(panel.getBoundingClientRect(), x, y) };
    }
    const area = el.closest<HTMLElement>(`[data-layout="${key}"]`);
    const tile = el.closest<HTMLElement>('[data-tile-id]');
    if (item.tile && tile && tile.dataset.tileId !== item.tile) {
      return { kind: 'tile', id: tile.dataset.tileId!, zone: zoneAt(tile.getBoundingClientRect(), x, y) };
    }
    if (area) return { kind: 'panel', layout: item.layout, panel: null, edge: edgeAt(area.getBoundingClientRect(), x, y) };
    return null;
  }
  const tab = el.closest<HTMLElement>('[data-tab-id]');
  if (tab) {
    const r = tab.getBoundingClientRect();
    const after = tab.dataset.vertical ? y > r.top + r.height / 2 : x > r.left + r.width / 2;
    return { kind: 'tab', id: tab.dataset.tabId!, after };
  }
  const group = el.closest<HTMLElement>('[data-group-id]');
  if (group) return { kind: 'group', id: group.dataset.groupId! };
  const bar = el.closest<HTMLElement>('[data-tab-bar]');
  // Tabs in a gap of a wrapped bar land next to the closest tab (V101-38).
  if (bar && kind === 'tab' && !bar.matches('[aria-orientation="vertical"]')) {
    const cells = [...bar.querySelectorAll<HTMLElement>('[data-tab-id]')].map((c) => ({
      id: c.dataset.tabId!,
      rect: c.getBoundingClientRect(),
    }));
    const slot = rowSlot(cells, x, y);
    if (slot) return { kind: 'tab', ...slot };
  }
  if (bar || el.closest('[data-drop-bar]')) return { kind: 'bar' };
  const tile = el.closest<HTMLElement>('[data-tile-id]');
  if (tile) return { kind: 'tile', id: tile.dataset.tileId!, zone: zoneAt(tile.getBoundingClientRect(), x, y) };
  return null;
}

function apply(item: DragItem, target: DropTarget): void {
  if (item.kind === 'tab') dropTab(item.id, target);
  else if (item.kind === 'tile') dropTile(item.id, target);
  else if (target.kind === 'panel') panelDrops.get(item.layout)?.(item.panel, target.panel, target.edge);
  else if (target.kind === 'tile' && item.tile) dropTile(item.tile, target);
}

function dropTab(id: string, target: DropTarget): void {
  const tab = tabs.tab(id);
  if (!tab) return;
  switch (target.kind) {
    case 'tab': {
      if (target.id === id) return;
      const rest = tabs.list.filter((t) => t.id !== id);
      const at = rest.findIndex((t) => t.id === target.id) + (target.after ? 1 : 0);
      const [prev, next] = [rest[at - 1], rest[at]];
      void tabs.apply({ op: 'move', tab: id, before: next?.id ?? null, group: groupAt(prev, next, tab.group) });
      return;
    }
    case 'group':
      void tabs.apply({ op: 'group', tabs: [id], group: target.id });
      return;
    case 'bar':
      void tabs.apply({ op: 'move', tab: id, before: null, group: null });
      return;
    case 'tile': {
      // A one-channel tab dropped on another tab's tile joins that tab (U3).
      const own = tiles(tab);
      if (own.length !== 1 || tabs.tabOf(target.id)?.id === id) return;
      void tabs.apply({ op: 'moveTile', tile: own[0].id, target: target.id, zone: target.zone });
    }
  }
}

function dropTile(id: string, target: DropTarget): void {
  switch (target.kind) {
    case 'tile':
      if (target.id !== id) void tabs.apply({ op: 'moveTile', tile: id, target: target.id, zone: target.zone });
      return;
    case 'tab':
      void tabs.apply({ op: 'sendTile', tile: id, tab: target.id });
      return;
    case 'bar':
      void tabs.apply({ op: 'detach', tile: id, before: null });
      return;
    case 'group':
  }
}
