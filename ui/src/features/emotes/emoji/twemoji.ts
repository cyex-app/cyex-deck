// V108-C30 Emoji drawn as Twemoji pictures, like Chatterino's default set. File names follow
// emoji-datasource-twitter: the fully qualified code points, lowercase, at least 4 digits,
// joined by `-` (`2764-fe0f`, `1f468-200d-1f469-200d-1f467`, `0023-fe0f-20e3`).

/** Pinned, so a package update never moves the pictures. */
export const TWEMOJI_VERSION = '15.1.2';
/** V108-F50 The 64 px set ships with the app (ui/public/twemoji, licence there): no CDN. */
const BASE = '/twemoji/64/';

const VS16 = 0xfe0f;
const PICTO = /^\p{Extended_Pictographic}$/u;
const PRESENTED = /^\p{Emoji_Presentation}$/u;
const MODIFIER = /^\p{Emoji_Modifier}$/u;

/** The picture's file name without extension, or null for an empty string. */
export function twemojiName(char: string): string | null {
  const cps = Array.from(char, (c) => c.codePointAt(0)!);
  const out: number[] = [];
  for (let i = 0; i < cps.length; i++) {
    const cp = cps[i];
    if (cp === VS16) continue;
    out.push(cp);
    const s = String.fromCodePoint(cp);
    // Keycaps keep the selector before U+20E3.
    if (cps[i + 1] === 0x20e3 || cps[i + 2] === 0x20e3) {
      if (/^[#*0-9]$/.test(s)) out.push(VS16);
      continue;
    }
    const next = cps[i + 1] === VS16 ? cps[i + 2] : cps[i + 1];
    const toned = next !== undefined && MODIFIER.test(String.fromCodePoint(next));
    if (PICTO.test(s) && !PRESENTED.test(s) && !toned) out.push(VS16);
  }
  if (!out.length) return null;
  return out.map((c) => c.toString(16).padStart(4, '0')).join('-');
}

/** The picture's address, or null when there is nothing to draw. */
export function twemojiUrl(char: string): string | null {
  const name = twemojiName(char);
  return name && `${BASE}${name}.png`;
}
