// V1051-03 How wide a paint cell is: 176 px, or wider when the account's name in that paint
// (bold 14 px, chat font) needs more. One width per name and font, so every cell is the same
// size and no screen stretches it.

export const MIN_PAINT_CELL = 176;

/** Cell padding around the text: .item 8 + 1 on both sides, .back 6 on both sides, and a few
 *  px so the check mark and rounding never touch the name. */
const PAD = 18 + 12 + 4;

/** The cell width for a name that measures `textWidth` px. */
export function paintCellWidth(textWidth: number): number {
  return Math.max(MIN_PAINT_CELL, Math.ceil(textWidth) + PAD);
}

const cache = new Map<string, number>();

/** The name's width in the paint cell's font, from an off-screen span (so CSS variables, the
 *  chosen chat font and its fallbacks all apply). Cached per name and resolved font. */
export function measureName(name: string): number {
  if (!name || typeof document === 'undefined') return 0;
  const span = document.createElement('span');
  span.style.cssText = 'position:absolute;visibility:hidden;white-space:pre;left:-9999px;top:0;font:700 14px var(--chat-font)';
  span.textContent = name;
  document.body.appendChild(span);
  const font = getComputedStyle(span).fontFamily;
  const key = `${font}|${name}`;
  let w = cache.get(key);
  if (w === undefined) {
    w = span.getBoundingClientRect().width;
    cache.set(key, w);
  }
  span.remove();
  return w;
}
