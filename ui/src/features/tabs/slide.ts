// V110-A-idea Tabs slide to fill the gap when one is closed with the mouse (FLIP): the
// positions before the close and after it give each tab's distance to travel.

export interface Spot {
  x: number;
  y: number;
}

/** Closer than this is not worth an animation. */
const MIN = 0.5;

/** For every tab in both lists that moved: how far it was from where it is now. */
export function slideDeltas(before: ReadonlyMap<string, Spot>, after: ReadonlyMap<string, Spot>): Map<string, Spot> {
  const out = new Map<string, Spot>();
  for (const [id, now] of after) {
    const was = before.get(id);
    if (!was) continue;
    const dx = was.x - now.x;
    const dy = was.y - now.y;
    if (Math.abs(dx) >= MIN || Math.abs(dy) >= MIN) out.set(id, { x: dx, y: dy });
  }
  return out;
}
