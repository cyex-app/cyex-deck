// V110-U52 "Above chat": the tab bar sits over the chat column inside the tab on screen,
// while that chat is beside its stream. `applies` says when. The parts only a measured tile
// knows (the chat turned under the stream to fit, a mod view, a hand-made arrangement) arrive
// through `mark`, kept per tab so switching back needs no second measuring.
// V110-U54 One bar (`AboveBar`, mounted once by the window) moves between the top and the
// slots the tiles register through the `BAR_SLOT` context (`BarSlots`).

import { untrack } from 'svelte';

import type { Tab } from '../../ipc/bindings/Tab';
import { settings } from '../../lib/settings.svelte';
import { chatBeside } from './barLayout';
import { chatSide } from './layout.svelte';
import { tabs, tiles } from './tabs.svelte';
import { theatre } from './theatre.svelte';

class AboveChat {
  /** Tabs whose tile turned the chat away from the side (measured). */
  private off = $state.raw<Set<string>>(new Set());

  mark(tab: string, away: boolean): void {
    // V110-F19 Marks stay while a tile is gone (coming back to a narrow tab must not move the
    // bar); they go with their tab, pruned here.
    const next = new Set([...this.off].filter((id) => tabs.list.some((t) => t.id === id)));
    if (away) next.add(tab);
    else next.delete(tab);
    if (next.size === this.off.size && [...next].every((id) => this.off.has(id))) return;
    this.off = next;
  }

  /** The setting is on, `tab` has one tile with its chat beside the stream, and the tile
   * hasn't turned it away. */
  applies(tab: Tab | undefined): boolean {
    if (!tab || settings.v.appearance.tabBarSide !== 'aboveChat' || this.off.has(tab.id)) return false;
    const all = tiles(tab);
    const t = all[0];
    return (
      !!t &&
      chatBeside({
        tiles: all.length,
        feed: !!t.feed,
        stream: t.show.player,
        chat: t.show.chat,
        chatPopped: tabs.chats.has(t.id),
        theatre: !!theatre.tile && all.some((x) => x.id === theatre.tile),
        side: chatSide(t),
      })
    );
  }
}

export const aboveChat = new AboveChat();

/** V110-U54 Context key: the window's `BarSlots`. Pop-outs have none, so their tiles draw no slot. */
export const BAR_SLOT = Symbol('barSlot');

/** V110-U54 Where each tab's tile wants the bar: an empty row at the top of its chat column,
 * kept `height` px tall so the chat doesn't move when the bar comes or goes. */
export class BarSlots {
  slots = $state.raw<ReadonlyMap<string, HTMLElement>>(new Map());
  /** The bar's last height in a slot (px). */
  height = $state(0);

  /** `{@attach slots.slot(tab.id)}` on the slot element. Reads untracked: the attachment
   * must not rerun on its own write. */
  slot(tab: string) {
    return (el: HTMLElement) => {
      untrack(() => (this.slots = new Map(this.slots).set(tab, el)));
      return () =>
        untrack(() => {
          if (this.slots.get(tab) !== el) return;
          const next = new Map(this.slots);
          next.delete(tab);
          this.slots = next;
        });
    };
  }
}
