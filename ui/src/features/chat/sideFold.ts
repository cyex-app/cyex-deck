// The message box's side buttons (channel points, Bits, chat identity, emotes) fold into one
// "…" button when the chat is narrow. Pure rules here; ChatInput measures and applies them.

/** Below this chat width the side buttons fold. */
export const FOLD_BELOW = 220;
/** The message box keeps at least this much. */
export const MIN_BOX = 120;
/** The `.input` side padding (8 + 8). */
export const PAD = 16;
/** One side button left of the box: 30 wide, 2 gap. */
const SIDE_W = 32;
/** Send button with its margin. */
const SEND_W = 38;
/** The box's own left margin. */
const GAP = 4;

/** `rowWidth`: the button row's width. `outside`: buttons left of the box (points, Bits).
 *  `send`: the Send button shows. Folds when the chat is under 220 px, or when the box
 *  would get under 120 px with those buttons in front of it. */
export function shouldFold(rowWidth: number, outside: number, send: boolean): boolean {
  if (rowWidth + PAD < FOLD_BELOW) return true;
  if (outside <= 0) return false;
  return rowWidth - outside * SIDE_W - (send ? SEND_W : 0) - GAP < MIN_BOX;
}
