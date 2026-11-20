// V1051-04 Tab after `/combo`, `/paint` and `/badge` completes their argument: combo names and
// sub-commands, paint and badge names. I106-3 `/remove` and `/rename` complete the first word only: the
// channel’s 7TV emote names. V111-03 `#` completes channels on all the 7TV emote commands. Ranked with the one helper (`complete`).

import { complete } from './completion.ts';

/** V112-17 cyex.tv's emote commands, and 7TV's with a 7 in front. */
type EmoteVerb = 'add' | 'remove' | 'rename' | 'steal' | 'yoink' | 'whoadded';
export type EmoteCommand = EmoteVerb | `7${EmoteVerb}`;

export interface ArgSpot {
  command: 'combo' | 'paint' | 'badge' | EmoteCommand;
  sub: string | null;
  start: number;
  typed: string;
  quote: boolean;
}

interface Arg {
  start: number;
  value: string;
  quote: boolean;
  closed: boolean;
}

/** `/combo` sub-commands. Never quoted. */
const SUBS = ['list', 'save', 'delete', 'rename'];
const COMMAND = /^\/(combo|paint|badge|7?add|7?remove|7?rename|7?steal|7?yoink|7?whoadded) /i;
const OPEN_QUOTE = /["“]/;
const CLOSE_QUOTE = /["”]/;

/** The words of `rest` (the text after the command), with where each starts in `rest`. */
function splitArgs(rest: string): Arg[] {
  const out: Arg[] = [];
  let i = 0;
  while (i < rest.length) {
    if (rest[i] === ' ') {
      i++;
      continue;
    }
    const start = i;
    const quote = OPEN_QUOTE.test(rest[i]);
    if (quote) i++;
    let value = '';
    let closed = false;
    for (; i < rest.length; i++) {
      const ch = rest[i];
      if (quote ? CLOSE_QUOTE.test(ch) : ch === ' ') {
        closed = true;
        break;
      }
      value += ch;
    }
    if (quote && closed) i++;
    out.push({ start, value, quote, closed });
  }
  return out;
}

/** I106-3 / V111-03 / V112-17 The emote commands, cyex.tv's and 7TV's (`/7add` …). */
const EMOTE_COMMANDS = new Set(['add', 'remove', 'rename', 'steal', 'yoink', 'whoadded'].flatMap((v) => [v, `7${v}`]));

/** V111-03 A 7TV emote command's word at the caret. A word starting with `#` completes channels
 * (`sub` '#', after the `#`); emote names have no spaces, so the name words are: every one for
 * /remove, the first for the others (none for /add). Their `sub` is the `#channel` typed
 * anywhere in the line (`whole`: the text after the command, caret or not), or null. */
function emoteArg(command: ArgSpot['command'], whole: string, from: number, rest: string): ArgSpot | null {
  const wordStart = rest.search(/\S*$/);
  const typed = rest.slice(wordStart);
  if (typed.startsWith('#')) return { command, sub: '#', start: from + wordStart + 1, typed: typed.slice(1), quote: false };
  if (command === 'add' || command === '7add') return null;
  const before = rest.slice(0, wordStart).split(/\s+/).filter((w) => w && !w.startsWith('#'));
  if (command !== 'remove' && command !== '7remove' && before.length > 0) return null;
  const channel = whole.split(/\s+/).find((w) => /^#[A-Za-z0-9_]{1,25}$/.test(w));
  return { command, sub: channel ? channel.slice(1).toLowerCase() : null, start: from + wordStart, typed, quote: false };
}

/** Where an argument is being typed, or null. `start` is where the replaced text begins
 * (an opening quote included), `typed` what's after the quote up to the caret. */
export function argAt(text: string, caret: number): ArgSpot | null {
  const before = text.slice(0, caret);
  const head = before.trimStart();
  const m = COMMAND.exec(head);
  if (!m) return null;
  const command = m[1].toLowerCase() as ArgSpot['command'];
  const from = before.length - head.length + m[0].length;
  const rest = head.slice(m[0].length);
  if (EMOTE_COMMANDS.has(command)) return emoteArg(command, text.slice(from), from, rest);
  if (command !== 'combo') return { command, sub: null, start: from, typed: rest, quote: false };

  const args = splitArgs(rest);
  const open = args.length > 0 && !args[args.length - 1].closed;
  // A closed quoted name with no space after it: nothing to complete.
  if (!open && rest && !/\s$/.test(rest)) return null;
  const index = open ? args.length - 1 : args.length;
  const at = open ? args[index] : { start: rest.length, value: '', quote: false, closed: false };
  const base = { command, start: from + at.start, typed: at.value, quote: at.quote };

  if (index === 0) return { ...base, sub: null };
  // Only the first word after `delete` or `rename` is a combo name.
  if (index === 1 && !args[0].quote) {
    const first = args[0].value.toLowerCase();
    if (first === 'delete' || first === 'rename') return { ...base, sub: first };
  }
  return null;
}

/** Quotes a combo name with a space, or any name the user opened a quote for. */
function quoted(spot: ArgSpot, value: string): string {
  return spot.command === 'combo' && !SUBS.includes(value) && (spot.quote || value.includes(' ')) ? `"${value}"` : value;
}

/** Ranked values to insert: names with spaces get quotes after /combo. */
export function argMatches(spot: ArgSpot, candidates: readonly string[], limit = 30): string[] {
  if (!spot.typed) {
    // V1051-05 Names first, then `list`, `save`…: `/combo ` offers your combos before the verbs.
    const names = spot.command === 'combo' && !spot.sub ? candidates.filter((v) => !SUBS.includes(v)) : [...candidates];
    const subs = candidates.length === names.length ? [] : candidates.filter((v) => SUBS.includes(v));
    return [...names, ...subs].slice(0, limit).map((v) => quoted(spot, v));
  }
  // The name typed exactly comes first, so `list` gets its space before Tab moves on.
  const exact = candidates.includes(spot.typed) ? [spot.typed] : [];
  return [...exact, ...complete(spot.typed, candidates, limit - exact.length)].map((v) => quoted(spot, v));
}
