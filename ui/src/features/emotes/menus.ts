// 1.40 Emote menu data per chat and account, shared by the menu and `:` completion.
// Reused for a minute; Rust caches the slow parts longer.

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import { emotesIpc } from '../../ipc/emotes';
import { dropStale, setCapped, useCapped } from '../../lib/capped';

const FRESH_MS = 60_000;
/** L1 Menus kept at most; each holds every emote of a channel and account. */
const MAX_MENUS = 12;

const cache = new Map<string, { at: number; menu: Promise<EmoteMenu> }>();

/** V106-31/32 After a 7TV edit: the next open asks again. */
export function forgetEmoteMenu(_login?: string): void {
  cache.clear();
}

export function emoteMenu(login: string, account: string | null): Promise<EmoteMenu> {
  const key = `${login}|${account ?? ''}`;
  const now = Date.now();
  // L1 Expired ones go, so a channel opened once doesn't keep its menu all run.
  dropStale(cache, (e) => now - e.at >= FRESH_MS);
  const hit = useCapped(cache, key);
  if (hit) return hit.menu;
  const menu = emotesIpc.menu(login, account);
  setCapped(cache, key, { at: now, menu }, MAX_MENUS);
  menu.catch(() => cache.get(key)?.menu === menu && cache.delete(key));
  return menu;
}
