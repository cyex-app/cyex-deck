// C6 What `:` and `!` suggest: emotes you can use here plus emoji (E1), and chat bot
// commands you're allowed to run (C13).

import type { BotCommand } from '../../ipc/bindings/BotCommand';
import type { ModRole } from '../../ipc/bindings/ModRole';
import type { Provider } from '../../ipc/bindings/Provider';
import { allows } from '../modview/roles.ts';
import { complete } from './completion.ts';

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

/** Emotes first, then emoji by short name. `used` as in `complete`. */
export function emoteItems(
  prefix: string,
  emotes: EmoteName[],
  emoji: EmojiName[],
  limit = 8,
  used?: (name: string) => number,
): Item[] {
  const byName = new Map(emotes.map((e) => [e.name, e]));
  const names = complete(prefix, byName.keys(), limit, used);
  const items: Item[] = names.map((n) => {
    const e = byName.get(n)!;
    return { value: n, label: n, emote: { provider: e.provider, id: e.id, animated: e.animated } };
  });
  for (const e of emoji) {
    if (items.length >= limit) break;
    items.push({ value: e.char, label: `:${e.name}:`, glyph: e.char });
  }
  return items;
}

/** C13 What a viewer may run: 0 everyone, 1 subs, 3 VIPs, 4 mods, 5 the streamer. */
export function viewerLevel(own: { role: ModRole | null; vip: boolean; sub: boolean } | null): number {
  if (!own) return 0;
  if (allows(own.role, 'broadcaster')) return 5;
  if (allows(own.role, 'mod')) return 4;
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
