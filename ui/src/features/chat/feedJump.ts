// V111-53 Mention tabs (Mentions and Global mentions) list the newest line at the top, and a
// line still in its chat's scrollback can be shown there ("Show in chat"); otherwise only
// the logs have it. Pure parts, so they test without a window.

import type { Tab } from '../../ipc/bindings/Tab';
import { feedKey, GLOBAL_FEED, isMentionsFeed } from './mentionsTab.ts';

/** Mention tabs read newest first; custom feeds keep chat order. */
export const newestFirst = (feed: string): boolean => isMentionsFeed(feed) || feedKey(feed) === feedKey(GLOBAL_FEED);

/** The lines in the order the tab draws them (`list` is oldest first, as Rust sends it). */
export function ordered<T>(list: readonly T[], newest: boolean): readonly T[] {
  return newest ? [...list].reverse() : list;
}

/** Where a channel's chat is in this window. */
export interface ChatSpot {
  tab: string;
  tile: string;
}

export interface Here {
  /** The tab on screen, if any. */
  active: string | null;
  /** Tabs in their own window. */
  popped: ReadonlySet<string>;
  /** Tiles whose chat is in its own window. */
  poppedChats: ReadonlySet<string>;
  /** Only this tab (a tab's own window; "" when the window has none), or null: any tab here (the main window). */
  only: string | null;
}

/** The chat tile showing `channel` in this window: the tab on screen first, then the first
 *  other tab. Null when no chat of it is here. */
export function chatSpot(list: readonly Tab[], channel: string, here: Here): ChatSpot | null {
  const login = channel.toLowerCase();
  const tabs = list.filter((t) => (here.only !== null ? t.id === here.only : !here.popped.has(t.id)));
  tabs.sort((a, b) => Number(b.id === here.active) - Number(a.id === here.active));
  for (const tab of tabs) {
    for (const col of tab.columns) {
      const tile = col.tiles.find((t) => !t.feed && t.show.chat && t.channel.login === login && !here.poppedChats.has(t.id));
      if (tile) return { tab: tab.id, tile: tile.id };
    }
  }
  return null;
}

const FRESH_MS = 3000;

/** "Show in chat" is offered when a chat of the channel is here and its room still has the
 *  line. One IPC call per line on hover or right-click, kept for a few seconds; never per draw. */
export function inChatCheck(spot: (channel: string) => ChatSpot | null, hasLine: (channel: string, id: string) => Promise<boolean>, now = () => Date.now()) {
  const seen = new Map<string, { ok: boolean; at: number }>();
  return async (channel: string, id: string): Promise<ChatSpot | null> => {
    const where = spot(channel);
    if (!where || !id || id.startsWith('local-')) return null;
    const key = `${channel}/${id}`;
    const hit = seen.get(key);
    if (hit && now() - hit.at < FRESH_MS) return hit.ok ? where : null;
    const ok = await hasLine(channel, id).catch(() => false);
    if (seen.size > 200) seen.clear();
    seen.set(key, { ok, at: now() });
    return ok ? where : null;
  };
}
