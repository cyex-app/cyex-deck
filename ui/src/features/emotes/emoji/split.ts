// V102-07 Finds emoji in chat text, so each can show its name on hover. Kept apart from the
// name list, which loads only when a name is first wanted.

export interface Piece {
  text: string;
  emoji: boolean;
}

/** Flags, keycaps, and pictographs with their skin tone, variation selector and ZWJ joins. */
const EMOJI =
  /\p{Regional_Indicator}{2}|[#*0-9]️?⃣|\p{Extended_Pictographic}(?:️|\p{Emoji_Modifier})?(?:‍\p{Extended_Pictographic}(?:️|\p{Emoji_Modifier})?)*/gu;
const ASCII = /^[\x00-\x7F]*$/;

/** Plain runs and emoji, in order. Text without emoji comes back as one piece. */
export function splitEmoji(text: string): Piece[] {
  if (ASCII.test(text)) return [{ text, emoji: false }];
  const out: Piece[] = [];
  let at = 0;
  for (const m of text.matchAll(EMOJI)) {
    const start = m.index ?? 0;
    if (start > at) out.push({ text: text.slice(at, start), emoji: false });
    out.push({ text: m[0], emoji: true });
    at = start + m[0].length;
  }
  if (at < text.length || !out.length) out.push({ text: text.slice(at), emoji: false });
  return out;
}

/** The form names are looked up by: no variation selectors or skin tones. */
export function emojiKey(char: string): string {
  return char.replace(/️|\p{Emoji_Modifier}/gu, '');
}
