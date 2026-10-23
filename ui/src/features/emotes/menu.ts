// 1.40 Emote menu data: lookups for favorites and most used (C12) and search.

import type { EmoteKey } from '../../ipc/bindings/EmoteKey';
import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { MenuEmote } from '../../ipc/bindings/MenuEmote';
import type { MenuSection } from '../../ipc/bindings/MenuSection';
import { RANK } from '../chat/completion.ts';
import { PROVIDER_NAMES, emoteKey } from './emotes.ts';
import { type Candidate, rankEmotes } from './ranking.ts';

export { usageRank } from './ranking.ts';

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

/** Emotes whose name contains `q`, locked ones last, in the one emote order (V107-R7
 *  ranking.ts): exact name, prefix before contains, then emotes you send most (`used`:
 *  their place in the most-used list, C12), then menu order. */
export function search(menu: EmoteMenu, q: string, limit = 200, used?: Map<string, number>): MenuEmote[] {
  const seen = new Set<string>();
  const all: Candidate<MenuEmote>[] = [];
  for (const s of sections(menu)) {
    for (const e of s.emotes) {
      const k = cellKey(e);
      if (seen.has(k)) continue;
      seen.add(k);
      all.push({ item: e, name: e.name, used: used?.get(emoteKey(e.provider, e.id)), locked: !!e.locked });
    }
  }
  return rankEmotes(q, all, RANK.contains).slice(0, limit);
}

/** V107-My Search results kept by platform (Twitch, 7TV, BTTV, FFZ), each in the ranked order
 *  of `search`, capped in total, empty platforms left out; `cut` is whether more than `limit`
 *  matched (the hint shows only then). */
export function searchCapped(menu: EmoteMenu, q: string, limit = 200, used?: Map<string, number>) {
  const all = search(menu, q, limit + 1, used);
  const found = all.slice(0, limit);
  const groups = PROVIDER_TABS.map((tab) => ({ tab, emotes: found.filter((m) => m.provider === tab) })).filter((g) => g.emotes.length);
  return { groups, cut: all.length > limit };
}
