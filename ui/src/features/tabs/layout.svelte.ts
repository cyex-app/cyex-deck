// V102-23 Panels inside a tab. Unlocked, a chat or a stream is dragged by its header and
// dropped on a side of the other one; each tile keeps its own chat side (stored with the
// tab, by its id), and Settings (2.1) stays what new tiles use. Being unlocked isn't
// saved: it's for arranging, and every tab starts locked.

import type { ChatPosition } from '../../ipc/bindings/ChatPosition';
import type { Tab } from '../../ipc/bindings/Tab';
import type { Tile } from '../../ipc/bindings/Tile';
import { settings } from '../../lib/settings.svelte';
import { tabs, tiles } from './tabs.svelte';

class Layout {
  /** Tabs whose panels can be dragged now. */
  unlocked = $state.raw<Set<string>>(new Set());

  isUnlocked(tab: string): boolean {
    return this.unlocked.has(tab);
  }

  setUnlocked(tab: string, on: boolean): void {
    const next = new Set(this.unlocked);
    if (on) next.add(tab);
    else next.delete(tab);
    this.unlocked = next;
  }

  /** Every chat in the tab back where Settings puts it, every mod view divider (V102-26) to half. */
  reset(tab: Tab): void {
    void tabs.apply({ op: 'resetLayout', tab: tab.id });
  }
}

export const layout = new Layout();

/** Where a tile's chat sits: dragged there, else as in Settings (2.1). */
export function chatSide(tile: Tile): ChatPosition {
  return tile.chatSide ?? settings.v.general.chatPosition;
}

/** Some chat in the tab was dragged away from Settings' side, or a mod view's divider moved (V102-26). */
export function dragged(tab: Tab): boolean {
  return tiles(tab).some((t) => (t.chatSide ?? null) !== null || (t.modviewShare ?? null) !== null);
}
