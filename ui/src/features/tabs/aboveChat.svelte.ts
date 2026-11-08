// V110-U52 "Above chat": the tab bar sits over the chat column inside the tab on screen,
// while that chat is beside its stream. The window (App) and the tile (TileView) both ask
// `applies`, so the bar is in one place at a time. The parts only a measured tile knows (the
// chat turned under the stream to fit, a mod view, a hand-made arrangement) arrive through
// `mark`, kept per tab so switching back needs no second measuring.

import type { Tab } from '../../ipc/bindings/Tab';
import { settings } from '../../lib/settings.svelte';
import { chatBeside } from './barLayout';
import { chatSide } from './layout.svelte';
import { tabs, tiles } from './tabs.svelte';

/** Context key: the window tells its tab views which tab is on screen and how to open one. */
export const BAR_HOST = Symbol('tabBarHost');
export interface BarHost {
  readonly current: string | null;
  open: (id: string) => void;
}

class AboveChat {
  /** Tabs whose tile turned the chat away from the side (measured). */
  private off = $state.raw<Set<string>>(new Set());

  mark(tab: string, away: boolean): void {
    if (this.off.has(tab) === away) return;
    const next = new Set(this.off);
    if (away) next.add(tab);
    else next.delete(tab);
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
        side: chatSide(t),
      })
    );
  }
}

export const aboveChat = new AboveChat();
