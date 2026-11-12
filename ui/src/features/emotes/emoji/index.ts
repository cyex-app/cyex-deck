// E1 Emoji for the emote menu and `:name:` completion, and V102-07 their names on hover.
// The list loads on first use.

import { emojiKey } from './split.ts';

export interface Emoji {
  char: string;
  /** Short names, first one is the main one. */
  names: string[];
  /** Extra search words. */
  words: string;
}

export interface EmojiGroup {
  title: string;
  emoji: Emoji[];
}

const TITLES: Record<string, string> = {
  'Smileys & Emotion': 'Smileys',
  'People & Body': 'People',
  'Animals & Nature': 'Nature',
  'Food & Drink': 'Food',
  'Travel & Places': 'Travel',
};

let groups: Promise<EmojiGroup[]> | null = null;

export function parseGroups(raw: [string, string][]): EmojiGroup[] {
  return raw.map(([category, lines]) => ({
    title: TITLES[category] ?? category,
    emoji: lines.split('\n').map((line) => {
      const space = line.indexOf(' ');
      const [names, words = ''] = line.slice(space + 1).split('|');
      const e = { char: line.slice(0, space), names: names.split(' '), words };
      return category === 'Flags' ? countryFlag(e) : e;
    }),
  }));
}

/** A country or region flag: two regional letters, or the black flag with tag letters (England). */
const COUNTRY = /^(?:[\u{1F1E6}-\u{1F1FF}]{2}|\u{1F3F4}[\u{E0061}-\u{E007A}]+\u{E007F})$/u;

/** V110-N17 Country flags are listed by country ("switzerland"), so `flag` found none of them:
 *  each also answers to `flag_<country>`, `flag_<code>` (`flag_ch`) and the word flag. */
function countryFlag(e: Emoji): Emoji {
  if (!COUNTRY.test(e.char)) return e;
  const country = e.names.find((n) => /[a-z]{3}/.test(n)) ?? e.words.replace(/^flag /, '').replaceAll(' ', '_');
  const code = Array.from(e.char, (c) => c.codePointAt(0)!)
    .filter((cp) => cp >= 0x1f1e6 && cp <= 0x1f1ff)
    .map((cp) => String.fromCharCode(cp - 0x1f1e6 + 97))
    .join('');
  const aliases = [country.startsWith('flag_') ? country : `flag_${country}`, code.length === 2 ? `flag_${code}` : ''];
  return {
    char: e.char,
    names: [...e.names, ...aliases.filter((a) => a && !e.names.includes(a))],
    words: /\bflag\b/.test(e.words) ? e.words : `${e.words} flag`.trim(),
  };
}

/** Every emoji by each of its names (lowercase), for `:name:` in a message. */
export function shortcodeMap(all: EmojiGroup[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const g of all) for (const e of g.emoji) for (const n of e.names) if (!map.has(n.toLowerCase())) map.set(n.toLowerCase(), e.char);
  return map;
}

/** V110-N17 `:name:` of a known emoji turns into the emoji when sent (`:flag_ch:`, `:sob:`), like
 *  Chatterino. Unknown names stay as typed; commands other than /me are left alone. */
export function expandShortcodes(text: string, map: Map<string, string>): string {
  if (text.startsWith('/') && !/^\/me\s/i.test(text)) return text;
  return text.replace(/:([a-z0-9_+-]{2,}):/gi, (all, name: string) => map.get(name.toLowerCase()) ?? all);
}

let codes: Promise<Map<string, string>> | null = null;

/** The `:name:` map, built once from the same list. */
export function emojiShortcodes(): Promise<Map<string, string>> {
  codes ??= emojiGroups().then(shortcodeMap);
  return codes;
}

export function emojiGroups(): Promise<EmojiGroup[]> {
  groups ??= import('./data').then((m) => parseGroups(m.EMOJI));
  return groups;
}

/** V102-07 What an emoji is called on hover: its short name, no colons ("eyes",
 * "heart_eyes"). A wordy name beats `+1`; flags read "flag_germany", not "de". */
export function emojiLabel(e: Emoji): string {
  const name = e.names.find((n) => /[a-z]{3}/.test(n));
  if (name) return name;
  return e.words ? e.words.replaceAll(' ', '_') : e.names[0];
}

/** Hover names by emoji, built from the same list. */
export function nameMap(all: EmojiGroup[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const g of all) {
    for (const e of g.emoji) {
      const key = emojiKey(e.char);
      if (!map.has(key)) map.set(key, emojiLabel(e));
    }
  }
  return map;
}

let names: Promise<Map<string, string>> | null = null;

/** The hover name for an emoji as found in chat, or null when it isn't in the list. */
export async function emojiName(char: string): Promise<string | null> {
  names ??= emojiGroups().then(nameMap);
  return (await names).get(emojiKey(char)) ?? null;
}

/** Names starting with `q` first, then ones containing it, then search words. */
export function findEmoji(all: EmojiGroup[], q: string, limit = 60): Emoji[] {
  const p = q.toLowerCase();
  if (!p) return [];
  // U4-5 Typing the emoji itself finds it (U+FE0F ignored).
  const bare = (s: string) => s.replaceAll('️', '');
  const typed = bare(q.trim());
  const exact: Emoji[] = [];
  const starts: Emoji[] = [];
  const contains: Emoji[] = [];
  const tagged: Emoji[] = [];
  for (const g of all) {
    for (const e of g.emoji) {
      if (typed && bare(e.char) === typed) exact.push(e);
      else if (e.names.some((n) => n.startsWith(p))) starts.push(e);
      else if (e.names.some((n) => n.includes(p))) contains.push(e);
      else if (e.words.includes(p)) tagged.push(e);
    }
  }
  return [...exact, ...starts,...contains, ...tagged].slice(0, limit);
}
