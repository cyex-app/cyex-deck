// Geometry for PositionPicker's little window: where the chat or tab bar block sits.

export type Side = 'left' | 'right' | 'top' | 'bottom';
export type Rect = { x: number; y: number; w: number; h: number };

/** The area under the title bar, in the picker's 56x38 drawing. */
export const CONTENT: Rect = { x: 1, y: 7, w: 54, h: 30 };

/** How thick the block is: chat is a wide column, the tab bar a thin strip. */
const THICK = {
  chat: { across: 18, along: 11 },
  tabs: { across: 6, along: 6 },
} as const;

/** The block on `side` and what is left for the player, both inside CONTENT. */
export function blocks(side: Side, kind: 'chat' | 'tabs'): { block: Rect; rest: Rect } {
  const { x, y, w, h } = CONTENT;
  const t = THICK[kind];
  switch (side) {
    case 'left':
      return { block: { x, y, w: t.across, h }, rest: { x: x + t.across, y, w: w - t.across, h } };
    case 'right':
      return { block: { x: x + w - t.across, y, w: t.across, h }, rest: { x, y, w: w - t.across, h } };
    case 'top':
      return { block: { x, y, w, h: t.along }, rest: { x, y: y + t.along, w, h: h - t.along } };
    case 'bottom':
      return { block: { x, y: y + h - t.along, w, h: t.along }, rest: { x, y, w, h: h - t.along } };
  }
}
