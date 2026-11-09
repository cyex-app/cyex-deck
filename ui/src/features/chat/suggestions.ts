// C6 What `:` and `!` suggest: emotes you can use here plus emoji (E1), and chat bot
// commands you're allowed to run (C13).

import type { BotCommand } from '../../ipc/bindings/BotCommand';
import type { ModRole } from '../../ipc/bindings/ModRole';
import type { Provider } from '../../ipc/bindings/Provider';
import { orderHits } from '../emotes/ranking.ts';
import { RANK } from './completion.ts';

export interface Item {
  /** What goes in the box. */
  value: string;
  label: string;
  /** Emote picture. */
  emote?: { provider: Provider; id: string; animated?: boolean };
  /** Emoji shown as text. */
  glyph?: string;
  hint?: string;
}

export interface EmoteName {
  name: string;
  provider: Provider;
  id: string;
  animated?: boolean;
}

export interface EmojiName {
  char: string;
  name: string;
}

/** Emotes and emoji in one order (V110-U71): the closest match first (the exact name, else the
 *  shortest that starts with what's typed, so `:sob` shows 😭 before SoBayed), the rest A-Z by how
 *  they match; emotes before emoji, an emoji first only when its name is exactly what's typed. Letters that merely come in order ("cdprSonbird" for
 *  `:sob`) aren't a match; acronyms are (V110-F49). Two emotes with one name: only the provider
 *  `provider` puts first (lower) is offered. Emoji found only by a search word come after. */
export function emoteItems(
  prefix: string,
  emotes: EmoteName[],
  emoji: EmojiName[],
  limit = 8,
  provider: (p: Provider) => number = () => 0,
): Item[] {
  const byName = new Map<string, EmoteName>();
  for (const e of emotes) {
    const have = byName.get(e.name);
    if (!have || provider(e.provider) < provider(have.provider)) byName.set(e.name, e);
  }
  type Hit = { emote: EmoteName } | { emoji: EmojiName };
  // Emotes before emoji; an emoji leads only when its name is exactly what's typed and no emote's is.
  const ems = orderHits<Hit>(prefix, [...byName.values()].map((e) => ({ item: { emote: e }, name: e.name, tie: provider(e.provider) })), RANK.acronym);
  const ejs = orderHits<Hit>(prefix, emoji.map((e) => ({ item: { emoji: e }, name: e.name })), RANK.acronym);
  const exact = (h: Hit | undefined) => !!h && ('emote' in h ? h.emote.name : h.emoji.name).toLowerCase() === prefix.trim().toLowerCase();
  const ranked = (exact(ejs[0]) && !exact(ems[0]) ? [ejs[0], ...ems, ...ejs.slice(1)] : [...ems, ...ejs]).slice(0, limit);
  const items: Item[] = ranked.map((h) =>
    'emote' in h
      ? { value: h.emote.name, label: h.emote.name, emote: { provider: h.emote.provider, id: h.emote.id, animated: h.emote.animated } }
      : { value: h.emoji.char, label: `:${h.emoji.name}:`, glyph: h.emoji.char },
  );
  for (const e of emoji) {
    if (items.length >= limit) break;
    if (!items.some((i) => i.glyph === e.char)) items.push({ value: e.char, label: `:${e.name}:`, glyph: e.char });
  }
  return items;
}

/** C13 What a viewer may run: 0 everyone, 1 subs, 3 VIPs, 4 mods, 5 the streamer. */
export function viewerLevel(own: { role: ModRole | null; vip: boolean; sub: boolean } | null): number {
  if (!own) return 0;
  if (own.role === 'broadcaster') return 5;
  if (own.role) return 4;
  if (own.vip) return 3;
  return own.sub ? 1 : 0;
}

export function commandItems(prefix: string, commands: BotCommand[], level: number, limit = 8): Item[] {
  const p = prefix.toLowerCase();
  return commands
    .filter((c) => c.level <= level && c.name.toLowerCase().startsWith(p) && c.name.toLowerCase() !== p)
    .slice(0, limit)
    .map((c) => ({
      value: c.name,
      label: c.name + (c.args.length ? ` ${c.args.map((a) => `<${a}>`).join(' ')}` : ''),
      hint: c.bot,
    }));
}
