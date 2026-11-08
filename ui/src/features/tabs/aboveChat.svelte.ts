// V110-U52 "Above chat": the tab bar sits over the chat column inside the tab on screen,
// while that chat is beside its stream. The window (App) and the tile (TileView) both ask
// `applies`, so the bar is in one place at a time. The parts only a measured tile knows (the
// chat turned under the stream to fit, a mod view, a hand-made arrangement) arrive through
// `mark`, kept per tab so switching back needs no second measuring.
// V110-F11 The bar never moves out from under the pointer: the tab changed while the pointer
// was on it (a wheel notch, a click, a close) holds the bar where it was until the pointer
// leaves it. The tab under it switches at once.

import type { Tab } from '../../ipc/bindings/Tab';
import { settings } from '../../lib/settings.svelte';
import { type AboveBar, aboveBar, type BarHold, chatBeside, inHold, type Rect, startsHold } from './barLayout';
import { chatSide } from './layout.svelte';
import { tabs, tiles } from './tabs.svelte';
import { theatre } from './theatre.svelte';

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
    // V110-F19 Marks stay while a tile is gone (coming back to a narrow tab must not move the
    // bar); they go with their tab, pruned here.
    const next = new Set([...this.off].filter((id) => tabs.list.some((t) => t.id === id)));
    if (away) next.add(tab);
    else next.delete(tab);
    if (next.size === this.off.size && [...next].every((id) => this.off.has(id))) return;
    this.off = next;
  }

  /** V110-F11 The bar's place and box while held, else null. */
  held = $state.raw<BarHold | null>(null);

  /** V110-F11 The tab on screen is about to change: when the pointer is on the bar, keep the
   * bar where it is now. Called before the page updates, so the old bar is still there. */
  hold(): void {
    if (this.held) return;
    const bar = [...document.querySelectorAll('[data-tab-bar]')].find((b) => b.matches(':hover'));
    const box = bar?.closest('[data-bar-box]') ?? bar;
    const r = box?.getBoundingClientRect();
    const rect: Rect | null = r ? { x: r.left, y: r.top, w: r.width, h: r.height } : null;
    if (!startsHold(settings.v.appearance.tabBarSide, !!bar, rect) || !rect || !box) return;
    this.held = { above: !!box.closest('[data-bar-above]'), rect };
  }

  /** V110-F11 The pointer left the held bar (or the window, or the window was resized). */
  release(x?: number, y?: number): void {
    const h = this.held;
    if (h && (x === undefined || y === undefined || !inHold(h.rect, x, y))) this.held = null;
  }

  /** V110-F19 A hold only makes sense with "Above chat" and no theatre mode: drop a stored one. */
  clearHeld(): void {
    this.held = null;
  }

  /** V110-F19 Once the new tab has drawn: the hold ends at once when that tab has no bar over a
   * chat (the bar would float over its stream) or its chat column isn't where the held bar is. */
  settle(tab: Tab | undefined): void {
    const h = this.held;
    if (!h?.above) return;
    if (!this.applies(tab)) {
      this.held = null;
      return;
    }
    const col = [...document.querySelectorAll('[data-chat-col]')]
      .map((c) => c.getBoundingClientRect())
      .find((r) => r.width > 0 && r.height > 0);
    if (col && (Math.abs(col.left - h.rect.x) > 2 || Math.abs(col.width - h.rect.w) > 2)) this.held = null;
  }

  /** V110-F11 How the bar is drawn for `tab` (see `aboveBar`). */
  bar(tab: Tab | undefined): AboveBar {
    const held = settings.v.appearance.tabBarSide === 'aboveChat' ? this.held : null;
    return aboveBar(this.applies(tab), held);
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
