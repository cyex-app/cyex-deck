// 1.40 Emote menu data: lookups for favorites and most used (C12) and search.

import type { EmoteKey } from '../../ipc/bindings/EmoteKey';
import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
import type { MenuSection } from '../../ipc/bindings/MenuSection';
import { emoteKey } from './emotes.ts';

export type ProviderTab = 'twitch' | 'seventv' | 'bttv' | 'ffz';

export const PROVIDER_TABS: ProviderTab[] = ['twitch', 'seventv', 'bttv', 'ffz'];

export function sections(menu: EmoteMenu): MenuSection[] {
  return PROVIDER_TABS.flatMap((t) => menu[t]);
}

/** Usable emotes by `provider:id`. */
export function usable(menu: EmoteMenu): Map<string, MenuEmote> {
  const map = new Map<string, MenuEmote>();
  for (const s of sections(menu)) {
    for (const e of s.emotes) {
      const k = emoteKey(e.provider, e.id);
      if (!e.locked && !map.has(k)) map.set(k, e);
    }
  }
  return map;
}

/** Saved emotes that can be used here, in saved order. */
export function pick(keys: EmoteKey[], have: Map<string, MenuEmote>): MenuEmote[] {
  return keys.map((k) => have.get(emoteKey(k.source, k.id))).filter((e): e is MenuEmote => !!e);
}

/** Usable emotes whose name starts with `q`, then ones containing it; locked ones last. */
export function search(menu: EmoteMenu, q: string, limit = 200): MenuEmote[] {
  const p = q.trim().toLowerCase();
  if (!p) return [];
  const seen = new Set<string>();
  const buckets: MenuEmote[][] = [[], [], [], []];
  for (const s of sections(menu)) {
    for (const e of s.emotes) {
      const k = emoteKey(e.provider, e.id);
      const name = e.name.toLowerCase();
      if (seen.has(k) || !name.includes(p)) continue;
      seen.add(k);
      buckets[(e.locked ? 2 : 0) + (name.startsWith(p) ? 0 : 1)].push(e);
    }
  }
  return buckets.flat().slice(0, limit);
}
