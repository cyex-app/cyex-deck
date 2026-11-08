// V110-C57 A text fragment cut where a highlight rule's matched words are (Rust says where:
// WordMark, UTF-16 units inside the fragment).

export interface Span {
  from: number;
  to: number;
}

export interface Piece {
  text: string;
  /** Inside a matched word. */
  hit: boolean;
}

/** `text` in pieces, the marked ones flagged. Marks out of range or overlapping are cut to fit. */
export function splitMarked(text: string, marks: readonly Span[]): Piece[] {
  const sorted = [...marks].sort((a, b) => a.from - b.from);
  const out: Piece[] = [];
  let at = 0;
  for (const m of sorted) {
    const from = Math.max(at, Math.min(m.from, text.length));
    const to = Math.min(text.length, m.to);
    if (to <= from) continue;
    if (from > at) out.push({ text: text.slice(at, from), hit: false });
    out.push({ text: text.slice(from, to), hit: true });
    at = to;
  }
  if (at < text.length) out.push({ text: text.slice(at), hit: false });
  return out.length ? out : [{ text, hit: false }];
}
