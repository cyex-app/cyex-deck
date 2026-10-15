// V105-13 review: Windows lists font families the webview can't draw by that name: weight and
// width names ("Segoe UI Semilight", "Bahnschrift Light Condensed"), names cut at 31
// characters, bitmap fonts (Fixedsys, Terminal). Chat would fall back to another font
// without a word, so the list leaves them out. A font draws when text in it has another
// width than in at least one of the generic fallbacks.

const SAMPLE = 'mmmmmmmmmmlliiWWwwQq@#1234567890';
const FALLBACKS = ['monospace', 'serif', 'sans-serif'];

let ctx: CanvasRenderingContext2D | null | undefined;
const widths = new Map<string, number>();
const known = new Map<string, boolean>();

function width(font: string): number {
  let w = widths.get(font);
  if (w === undefined && ctx) {
    ctx.font = `40px ${font}`;
    w = ctx.measureText(SAMPLE).width;
    widths.set(font, w);
  }
  return w ?? 0;
}

/** Whether the webview draws the installed font `name` (cached per name). */
export function draws(name: string): boolean {
  let hit = known.get(name);
  if (hit !== undefined) return hit;
  ctx ??= document.createElement('canvas').getContext('2d');
  if (!ctx) return true;
  const face = `"${name.replace(/["\\]/g, '')}"`;
  hit = FALLBACKS.some((f) => width(`${face}, ${f}`) !== width(f));
  known.set(name, hit);
  return hit;
}
