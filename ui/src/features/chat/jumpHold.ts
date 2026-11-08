// V110-F43 A jump to a line (a scrollbar mark, a reply, Jump to message) holds the line in the
// middle of the list while the layout settles. One scroll wasn't enough in a busy chat: lines
// drawn for the jump (drawAround) and lines coming into view (content-visibility) get their
// real height only after the scroll, the pill's strip comes in as the list leaves the bottom,
// and history drops off the top. Each moved the line, sometimes off screen.

/** The longest a jump holds its line, in ms. */
export const HOLD_MS = 2000;
/** It lets go once nothing moved the line for this long, in ms. */
export const SETTLE_MS = 400;

type Box = { top: number; height: number };

/** px the list scrolls so the line sits in its middle; positive is down. */
export function centreShift(view: Box, line: Box): number {
  return line.top + line.height / 2 - (view.top + view.height / 2);
}

/** Whether a jump made at `start` still holds its line, last nudged at `moved`. */
export function holding(now: number, start: number, moved: number): boolean {
  return now - start < HOLD_MS && now - moved < SETTLE_MS;
}
