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

/** Where the box goes: under the button, kept inside the window; above it when the bottom is short. */
export function warnSpot(
  button: { left: number; top: number; bottom: number },
  box: { width: number; height: number },
  view: { width: number; height: number },
): Spot {
  const gap = 4;
  const left = Math.max(gap, Math.min(button.left, view.width - box.width - gap));
  const below = button.bottom + gap;
  const top = below + box.height + gap <= view.height ? below : Math.max(gap, button.top - box.height - gap);
  return { left, top };
}
