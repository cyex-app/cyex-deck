// 1.40 Emote menu data: lookups for favorites and most used (C12) and search.

import type { EmoteKey } from '../../ipc/bindings/EmoteKey';
import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
import type { MenuSection } from '../../ipc/bindings/MenuSection';
import { PROVIDER_NAMES, emoteKey } from './emotes.ts';

export type ProviderTab = 'twitch' | 'seventv' | 'bttv' | 'ffz';

export const PROVIDER_TABS: ProviderTab[] = ['twitch', 'seventv', 'bttv', 'ffz'];

/** V105-04 A cell's key: one emote can sit in a set under two names. */
export function cellKey(m: MenuEmote): string {
  return `${emoteKey(m.provider, m.id)}:${m.name}`;
}

/** 1.40 The tier mark on a Twitch sub emote's tile: none for tier 1. */
export function tierMark(m: MenuEmote): string | null {
  return m.provider === 'twitch' && m.tier && m.tier > 1 ? `T${m.tier}` : null;
}

/** The tip's second line: why it's locked (E5), its tier, or where it's from. */
export function cellHint(m: MenuEmote): string {
  if (m.locked) return m.locked;
  return tierMark(m) ? `Tier ${m.tier} emote` : PROVIDER_NAMES[m.provider];
}

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

/** Usable emotes whose name starts with `q`, then ones containing it; locked ones last.
 *  Within each group the exact name leads, then emotes you send most (`used`: their place
 *  in the most-used list, C12), then menu order. */
export function search(menu: EmoteMenu, q: string, limit = 200, used?: Map<string, number>): MenuEmote[] {
  const p = q.trim().toLowerCase();
  if (!p) return [];
  const seen = new Set<string>();
  const buckets: MenuEmote[][] = [[], [], [], []];
  for (const s of sections(menu)) {
    for (const e of s.emotes) {
      const k = cellKey(e);
      const name = e.name.toLowerCase();
      if (seen.has(k) || !name.includes(p)) continue;
      seen.add(k);
      buckets[(e.locked ? 2 : 0) + (name.startsWith(p) ? 0 : 1)].push(e);
    }
  }
  const score = (e: MenuEmote) =>
    e.name.toLowerCase() === p ? -1 : (used?.get(emoteKey(e.provider, e.id)) ?? Number.MAX_SAFE_INTEGER);
  // Array sort is stable, so equal scores keep menu order.
  return buckets.flatMap((b) => b.sort((a, c) => score(a) - score(c))).slice(0, limit);
}

/** C12 Each emote's place in the most-used list, by `provider:id`. */
export function usageRank(usage: readonly { emote: EmoteKey }[]): Map<string, number> {
  return new Map(usage.map((u, i) => [emoteKey(u.emote.source, u.emote.id), i]));
}
