// The message box's side buttons (channel points, Bits, chat identity, emotes) fold into one
// "…" button when the chat is narrow. Pure rules here; ChatInput measures and applies them.

/** Below this chat width the side buttons fold. */
export const FOLD_BELOW = 220;
/** The text area keeps at least this much (the buttons inside the box not counted). */
export const MIN_BOX = 140;
/** The `.input` side padding (8 + 8). */
export const PAD = 16;
/** One side button left of the box: 30 wide, 2 gap. */
const SIDE_W = 32;
/** Send button with its margin. */
const SEND_W = 38;
/** The box's own left margin. */
const GAP = 4;
/** One button inside the box (emotes, identity, quick replies). */
const INNER_W = 30;

/** `rowWidth`: the button row's width. `outside`: buttons left of the box (points, Bits).
 *  `send`: the Send button shows. `inside`: buttons inside the box (emotes, identity, quick
 *  replies). Folds when the chat is under 220 px, or when the text area would get under
 *  140 px with those buttons around it. */
export function shouldFold(rowWidth: number, outside: number, send: boolean, inside = 0): boolean {
  if (rowWidth + PAD < FOLD_BELOW) return true;
  if (outside <= 0 && inside <= 0) return false;
  return rowWidth - outside * SIDE_W - inside * INNER_W - (send ? SEND_W : 0) - GAP < MIN_BOX;
}
