// 1.40 Emote menu data per chat and account, shared by the menu and `:` completion.
// Reused for a minute; Rust caches the slow parts longer.

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import { emotesIpc } from '../../ipc/emotes';

const FRESH_MS = 60_000;

const cache = new Map<string, { at: number; menu: Promise<EmoteMenu> }>();

/** V106-31/32 After a 7TV edit: the next open asks again. */
export function forgetEmoteMenu(login: string): void {
  for (const key of [...cache.keys()]) if (key.startsWith(`${login}|`)) cache.delete(key);
}

export function emoteMenu(login: string, account: string | null): Promise<EmoteMenu> {
  const key = `${login}|${account ?? ''}`;
  const hit = cache.get(key);
  if (hit && Date.now() - hit.at < FRESH_MS) return hit.menu;
  const menu = emotesIpc.menu(login, account);
  cache.set(key, { at: Date.now(), menu });
  menu.catch(() => cache.delete(key));
  return menu;
}
