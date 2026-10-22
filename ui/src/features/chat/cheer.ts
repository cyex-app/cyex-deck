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
