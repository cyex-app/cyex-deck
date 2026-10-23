// V107-Bn Cheer words in the chat box, by Twitch's rule `^([A-Za-z0-9]+[A-Za-z])(\d+)$`.
// The box uses this to pick the cheer path and to show the total; Rust checks again.

export interface CheerWord {
  /** Lowercase. */
  prefix: string;
  amount: number;
}

const WORD = /^([A-Za-z0-9]+[A-Za-z])(\d+)$/;

/** A slash or dot command, never a cheer (`/me` and `.me` are). */
export function isCommand(text: string): boolean {
  const t = text.trimStart();
  return (t.startsWith('/') || t.startsWith('.')) && !/^[/.]me\s/i.test(t);
}

/** The cheer words whose prefix `known` takes (lowercase). */
export function cheerWords(text: string, known: (prefix: string) => boolean): CheerWord[] {
  const out: CheerWord[] = [];
  for (const w of text.split(/\s+/)) {
    const m = WORD.exec(w);
    if (!m) continue;
    const prefix = m[1].toLowerCase();
    if (known(prefix)) out.push({ prefix, amount: Number(m[2]) });
  }
  return out;
}

/** Bits the text would spend; 0 for a command or no cheer. */
export function cheerTotal(text: string, known: (prefix: string) => boolean): number {
  if (isCommand(text)) return 0;
  return cheerWords(text, known).reduce((n, w) => n + w.amount, 0);
}

/** The picker's pick: the first `Prefix…` word gets the new amount, else it goes at the end. */
export function setCheer(text: string, prefix: string, amount: number): string {
  const word = `${prefix}${amount}`;
  const parts = text.split(/(\s+)/);
  const at = parts.findIndex((p) => {
    const m = WORD.exec(p);
    return !!m && m[1].toLowerCase() === prefix.toLowerCase();
  });
  if (at >= 0) {
    parts[at] = word;
    return parts.join('');
  }
  const head = text.trimEnd();
  return head ? `${head} ${word} ` : `${word} `;
}

// V107-H10a The picker: an amount first (presets or any number), then the style.

export const CHEER_PRESETS = [1, 100, 500, 1000, 5000, 10000] as const;
export const CHEER_MAX = 100_000;

export interface CheerRules {
  cheerMin: number;
  emoteMin: number;
  /** Null when Twitch didn't say. */
  balance: number | null;
}

/** The fewest Bits one cheer word may carry (per-emote minimum). */
export function wordMin(r: CheerRules): number {
  return Math.min(CHEER_MAX, Math.max(1, r.emoteMin));
}

/** The fewest Bits a cheer may spend: both channel minimums. */
export function cheerFloor(r: CheerRules): number {
  return Math.min(CHEER_MAX, Math.max(1, r.cheerMin, r.emoteMin));
}

/** A word of `bits` can go out: at least the per-emote minimum, at most the balance. */
export function canAfford(bits: number, r: CheerRules): boolean {
  return Number.isInteger(bits) && bits >= wordMin(r) && bits <= CHEER_MAX && (r.balance === null || bits <= r.balance);
}

export interface Preset {
  bits: number;
  /** False: greyed, more than you have. */
  ok: boolean;
}

/** Presets at or over the channel minimum; the minimum itself leads when it isn't one. */
export function cheerPresets(r: CheerRules): Preset[] {
  const floor = cheerFloor(r);
  const list: number[] = CHEER_PRESETS.filter((b) => b >= floor);
  if (list[0] !== floor) list.unshift(floor);
  return list.map((bits) => ({ bits, ok: canAfford(bits, r) }));
}

/** Where the amount starts: never under the channel minimum. */
export function startAmount(current: number, r: CheerRules): number {
  return Math.max(current, cheerFloor(r));
}
