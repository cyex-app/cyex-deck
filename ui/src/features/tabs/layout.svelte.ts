// V102-23 Panels inside a tab. Unlocked, a chat or a stream is dragged by its header and
// dropped on a side of the other one; each tile keeps its own chat side (stored with the
// tab, by its id), and Settings (2.1) stays what new tiles use. Being unlocked isn't
// saved: it's for arranging, and every tab starts locked. V103-05 Any panel of a tile
// (stream, chat, mod view) moves the same way, through the one panel layout
// (`components/PanelLayout.svelte`, `lib/panels.ts`) the mod view and the stream manager
// use too. V103-07 A mod view's stream panel can show its tile's own player: the slot it
// offers is kept here, by tile.

import type { ChatPosition } from '../../ipc/bindings/ChatPosition';
import type { Tab } from '../../ipc/bindings/Tab';
import type { Tile } from '../../ipc/bindings/Tile';
import { settings } from '../../lib/settings.svelte';
import { tabs, tiles } from './tabs.svelte';

class Layout {
  /** Tabs whose panels can be dragged now. */
  unlocked = $state.raw<Set<string>>(new Set());
  /** V103-07 Tile id → the element in its mod view where its player should show. */
  slots = $state.raw<Map<string, HTMLElement>>(new Map());
  /** Goes up whenever a panel layout lays itself out again, so a player shown in a slot
   * follows the slot when it moves without changing size. */
  moved = $state(0);

  isUnlocked(tab: string): boolean {
    return this.unlocked.has(tab);
  }

  setUnlocked(tab: string, on: boolean): void {
    const next = new Set(this.unlocked);
    if (on) next.add(tab);
    else next.delete(tab);
    this.unlocked = next;
  }

  /** Every chat in the tab back where Settings puts it, every mod view divider (V102-26)
   * to half, every hand-made arrangement (V103-05) gone. */
  reset(tab: Tab): void {
    void tabs.apply({ op: 'resetLayout', tab: tab.id });
  }

  /** V103-07 A mod view offers its tile's player a place. */
  offer(tile: string, el: HTMLElement): void {
    this.slots = new Map(this.slots).set(tile, el);
  }

  /** V103-07 It takes the place back. */
  withdraw(tile: string, el: HTMLElement): void {
    if (this.slots.get(tile) !== el) return;
    const next = new Map(this.slots);
    next.delete(tile);
    this.slots = next;
  }
}

export const layout = new Layout();

/** Where a tile's chat sits: dragged there, else as in Settings (2.1). */
export function chatSide(tile: Tile): ChatPosition {
  return tile.chatSide ?? settings.v.general.chatPosition;
}

/** Some chat in the tab was dragged away from Settings' side, a mod view's divider moved
 * (V102-26), or panels were arranged by hand (V103-05). */
export function dragged(tab: Tab): boolean {
  return tiles(tab).some((t) => (t.chatSide ?? null) !== null || (t.modviewShare ?? null) !== null || (t.layout ?? null) !== null);
}
