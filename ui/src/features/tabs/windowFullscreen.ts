// V110-B7 Who wants the main window fullscreen: a player's own fullscreen (F) and theatre.
// The window follows `wantFullscreen`; leaving theatre puts it back to maximized or normal
// size (tao restores the placement, `restorePlan` checks it did).

import { getCurrentWindow } from '@tauri-apps/api/window';

import { restorePlan, wantFullscreen, type WindowBefore } from './theatreWindow';

let player = false;
let theatre = false;
let before: WindowBefore | null = null;
let queue: Promise<void> = Promise.resolve();

async function apply(): Promise<void> {
  const win = getCurrentWindow();
  const want = wantFullscreen(player, theatre);
  const now = await win.isFullscreen();
  if (want && !now) {
    if (theatre && !before) before = { maximized: await win.isMaximized() };
    await win.setFullscreen(true);
  } else if (!want && now) {
    await win.setFullscreen(false);
    if (before) {
      const plan = restorePlan(before, await win.isMaximized());
      if (plan === 'maximize') await win.maximize();
      else if (plan === 'unmaximize') await win.unmaximize();
    }
  }
  if (!theatre) before = null;
}

function sync(): void {
  queue = queue.then(apply).catch(() => {});
}

export const windowFullscreen = {
  /** A player's fullscreen began or ended. */
  player(on: boolean): void {
    player = on;
    sync();
  },
  /** Theatre began or ended. */
  theatre(on: boolean): void {
    if (theatre === on) return;
    theatre = on;
    sync();
  },
};
