// Dragging tabs and tiles (U3, U5) with pointer events: reorder tabs, tile one tab's
// channel into another, move tiles around or out into their own tab.

import type { Zone } from '../../ipc/bindings/Zone';
import { tabs, tiles } from './tabs.svelte';
import { groupAt, rowSlot, zoneAt } from './zones';

export type DragItem = { kind: 'tab'; id: string } | { kind: 'tile'; id: string };

export type DropTarget =
  /** Next to a tab in the bar, or into it for a tile. */
  | { kind: 'tab'; id: string; after: boolean }
  | { kind: 'group'; id: string }
  /** The bar's empty end. */
  | { kind: 'bar' }
  | { kind: 'tile'; id: string; zone: Zone };

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
      this.target = hit(ev.clientX, ev.clientY, item.kind);
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

function hit(x: number, y: number, kind: DragItem['kind']): DropTarget | null {
  const el = document.elementFromPoint(x, y);
  if (!el) return null;
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
  else dropTile(item.id, target);
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
