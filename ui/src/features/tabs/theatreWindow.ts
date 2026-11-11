// V110-B7 Theatre is fullscreen: the window goes fullscreen with it and comes back to what it
// was. Pure rules; `windowFullscreen.ts` does the calls.

/** What the window was when theatre began. */
export interface WindowBefore {
  maximized: boolean;
}

/** The window is fullscreen while a player is, or theatre is on (a player's fullscreen ending must not end theatre's). */
export function wantFullscreen(playerFullscreen: boolean, theatreOn: boolean): boolean {
  return playerFullscreen || theatreOn;
}

/** After leaving fullscreen: what puts the window back to maximized or normal size. `null` when it already is. */
export function restorePlan(before: WindowBefore, maximizedNow: boolean): 'maximize' | 'unmaximize' | null {
  if (before.maximized === maximizedNow) return null;
  return before.maximized ? 'maximize' : 'unmaximize';
}
