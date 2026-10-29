// V108-N08 Your own colour for a person's name, nudged until it reads (4.5:1 on the page and
// the raised surface) like Twitch's name colours. Same steps as `readable_name` in
// crates/core/src/color.rs, on the built-in backgrounds (a custom theme's own page is only
// approximated, as with theme previews). Keep the two in sync.

import type { NameColor } from '../../ipc/bindings/NameColor';

type Rgb = [number, number, number];

const TEXT_CONTRAST = 4.5;
const DARK: Rgb[] = [
  [0x1e, 0x1f, 0x23],
  [0x26, 0x27, 0x2c],
];
const LIGHT: Rgb[] = [
  [0xf3, 0xf3, 0xf4],
  [0xff, 0xff, 0xff],
];

function parse(hex: string): Rgb | null {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function luminance([r, g, b]: Rgb): number {
  const ch = (c: number) => {
    const v = c / 255;
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b);
}

export function contrast(a: Rgb, b: Rgb): number {
  const [la, lb] = [luminance(a), luminance(b)];
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

const hex = (c: Rgb) => `#${c.map((v) => v.toString(16).padStart(2, '0')).join('')}`;

/** `color` pushed toward `away` in 5 % steps until it reads on every background. */
function readable(color: Rgb, away: Rgb, backgrounds: Rgb[]): Rgb {
  for (let step = 0; step <= 20; step++) {
    const t = step * 0.05;
    const c = color.map((v, i) => Math.round(v + (away[i] - v) * t)) as Rgb;
    if (backgrounds.every((bg) => contrast(c, bg) >= TEXT_CONTRAST)) return c;
  }
  return away;
}

/** The dark and light variants of `#rrggbb`, or null when it isn't one (their own colour shows). */
export function ownNameColor(color: string | null | undefined): NameColor | null {
  const rgb = color ? parse(color) : null;
  if (!rgb) return null;
  return {
    dark: hex(readable(rgb, [255, 255, 255], DARK)),
    light: hex(readable(rgb, [0, 0, 0], LIGHT)),
  };
}
