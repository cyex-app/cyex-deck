// C6 What `:` and `!` suggest: emotes you can use here plus emoji (E1), and chat bot
// commands you're allowed to run (C13).

import type { BotCommand } from '../../ipc/bindings/BotCommand';
import type { ModRole } from '../../ipc/bindings/ModRole';
import type { Provider } from '../../ipc/bindings/Provider';
import { rankEmotes } from '../emotes/ranking.ts';

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

/** Emotes and emoji in one order (V107-R7): how well the name matches first, so `:sob`
 *  shows 😭 before SoBayed and CoolStoryBob (V110-F10); on a tie emotes go first. Emoji found
 *  only by a search word come after. */
export function emoteItems(
  prefix: string,
  emotes: EmoteName[],
  emoji: EmojiName[],
  limit = 8,
  used?: (name: string) => number,
): Item[] {
  const byName = new Map(emotes.map((e) => [e.name, e]));
  type Hit = { emote: EmoteName } | { emoji: EmojiName };
  // Unlike Tab, `:` keeps the exact name: it ranks first (an exact match beats every prefix match).
  const ranked = rankEmotes<Hit>(prefix, [
    ...[...byName.values()].map((e) => ({ item: { emote: e }, name: e.name, used: used?.(e.name) })),
    ...emoji.map((e) => ({ item: { emoji: e }, name: e.name })),
  ]).slice(0, limit);
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
