// V108-N11 The colour a background tab's mark takes from the latest highlight rule that
// matched in it (Rust sends `#rrggbb` per room; empty when the rule has none).

const HEX = /^#[0-9a-f]{6}$/i;

/** The last valid colour of a tab's rooms, or null for the plain mark. */
export function markColor(colors: (string | null | undefined)[]): string | null {
  let out: string | null = null;
  for (const c of colors) if (c && HEX.test(c)) out = c;
  return out;
}
