// V1041-01 Tab completion for `/` commands: the message's first word, while the caret is in
// it. Ranked with the emote completion's helper (V101-16); which commands a chat can use
// comes from Rust. No list, no arguments.

import { COMMAND_HINTS } from './commandHints.ts';
import { complete, type Word } from './completion.ts';

/** The command name typed so far when `word` is the message's first word and starts with
 * `/`, else null. Spaces in front don't count; the line runs as a command with them too. */
export function commandAt(text: string, word: Word): string | null {
  if (text.slice(0, word.start).trim() || !word.text.startsWith('/')) return null;
  return word.text.slice(1);
}

/** `/name` for each of `names` that matches `typed`, best first. The name typed exactly
 * comes first, so `/ban` gets its space before Tab moves on to `/unban`. */
export function commandMatches(typed: string, names: readonly string[], limit = 30): string[] {
  if (!typed) return [];
  const exact = names.includes(typed) ? [typed] : [];
  return [...exact, ...complete(typed, names, limit - exact.length)].map((n) => `/${n}`);
}

/** A4 The one-line hint for a command (its first sentence), if the registry has one. */
export function commandHint(name: string): string | undefined {
  const s = COMMAND_HINTS[name.toLowerCase()];
  return s?.match(/^.*?[.!?](?=\s|$)/)?.[0] ?? s;
}
