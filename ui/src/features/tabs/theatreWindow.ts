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

/** V110-B10 Theatre is only ever fullscreen: a window that stops being fullscreen while it is on
 *  (Restore, Win+Down, a monitor change) ends it. The window counts as fullscreen from the first
 *  time it is seen so, so the moment before our own fullscreen call lands is not a leave. */
export interface FullscreenWatch {
  seen: boolean;
}

export interface WindowState {
  theatre: boolean;
  /** The window's own fullscreen state (the taskbar-room resize keeps it). */
  fullscreen: boolean;
  minimized: boolean;
}

export const UNSEEN: FullscreenWatch = { seen: false };

export function fullscreenWatch(prev: FullscreenWatch, w: WindowState): { next: FullscreenWatch; leave: boolean } {
  if (!w.theatre) return { next: UNSEEN, leave: false };
  // Minimize keeps theatre; coming back shows it fullscreen again.
  if (w.minimized) return { next: prev, leave: false };
  if (w.fullscreen) return { next: { seen: true }, leave: false };
  return prev.seen ? { next: UNSEEN, leave: true } : { next: prev, leave: false };
}

/** V110-B10 The main window's Maximize / Restore button. In theatre it reads Restore and leaves
 *  theatre (and so fullscreen) instead of toggling the maximized state. */
export function sizeButton(theatre: boolean, maximized: boolean): { icon: 'restore' | 'maximize'; label: 'Restore' | 'Maximize'; leavesTheatre: boolean } {
  if (theatre) return { icon: 'restore', label: 'Restore', leavesTheatre: true };
  return maximized ? { icon: 'restore', label: 'Restore', leavesTheatre: false } : { icon: 'maximize', label: 'Maximize', leavesTheatre: false };
}

/** V110-N5 R2-1 A page that starts (or reloads) while the window is fullscreen: if neither theatre nor
 *  a player wants it, the window is left over from the page before and goes back. */
export function bootPlan(isFullscreen: boolean, wantsTheatre: boolean, wantsPlayer: boolean): 'leave' | null {
  return isFullscreen && !wantFullscreen(wantsPlayer, wantsTheatre) ? 'leave' : null;
}
