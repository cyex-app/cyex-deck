// V110-A29 The window pin: one label ("Pin on top") with a pressed state; only the hint changes.
export const PIN_LABEL = 'Pin on top';

export function pinHint(pinned: boolean): string {
  return pinned ? 'Pinned. Click to let other windows cover it.' : 'Keeps this window above everything else.';
}
