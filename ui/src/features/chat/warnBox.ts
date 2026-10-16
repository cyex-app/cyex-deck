// M10 The reason box a chat line's Warn button opens. Twitch needs a reason to warn
// (Helix `reason` is 1 to 500 characters), so an empty one is never sent.

export const MAX_REASON = 500;

/** The `/warn` line for a reason, or null while there is nothing to send. */
export function warnCommand(login: string, reason: string): string | null {
  const r = reason.trim().slice(0, MAX_REASON);
  return login && r ? `/warn ${login} ${r}` : null;
}

export interface Spot {
  left: number;
  top: number;
}

/** Where the box goes. With the button's right edge known it sits beside the button (right
 *  of it, else left), centred on it, so it covers the line's own text and not the buttons of
 *  the lines around. Otherwise under the button, or above when the bottom is short. */
export function warnSpot(
  button: { left: number; top: number; bottom: number; right?: number },
  box: { width: number; height: number },
  view: { width: number; height: number },
): Spot {
  const gap = 4;
  const clampTop = (t: number) => Math.max(gap, Math.min(t, view.height - box.height - gap));
  if (button.right !== undefined) {
    const mid = (button.top + button.bottom) / 2;
    const right = button.right + gap;
    const left = right + box.width + gap <= view.width ? right : Math.max(gap, button.left - box.width - gap);
    return { left, top: clampTop(mid - box.height / 2) };
  }
  const left = Math.max(gap, Math.min(button.left, view.width - box.width - gap));
  const below = button.bottom + gap;
  const top = below + box.height + gap <= view.height ? below : Math.max(gap, button.top - box.height - gap);
  return { left, top };
}
