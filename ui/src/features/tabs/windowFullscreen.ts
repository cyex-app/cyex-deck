// V110-B7 Who wants the main window fullscreen: a player's own fullscreen (F) and theatre.
// The window follows `wantFullscreen`; leaving theatre puts it back to maximized or normal
// size (tao restores the placement, `restorePlan` checks it did).

import { getCurrentWindow } from '@tauri-apps/api/window';

import { fullscreenWatch, restorePlan, UNSEEN, wantFullscreen, type WindowBefore } from './theatreWindow';

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
  /** V110-B10 While theatre is on: `onLeave` runs when the window stops being fullscreen by
   *  itself (Restore, Win+Down, a monitor change). Returns the stop function. */
  watch(onLeave: () => void): () => void {
    const win = getCurrentWindow();
    let state = UNSEEN;
    let live = true;
    const check = async () => {
      const [fullscreen, minimized] = await Promise.all([win.isFullscreen(), win.isMinimized()]);
      if (!live) return;
      const { next, leave } = fullscreenWatch(state, { theatre: true, fullscreen, minimized });
      state = next;
      if (leave) onLeave();
    };
    const off = win.onResized(() => void check().catch(() => {}));
    return () => {
      live = false;
      void off.then((f) => f());
    };
  },
  /** Theatre began or ended. */
  theatre(on: boolean): void {
    if (theatre === on) return;
    theatre = on;
    sync();
  },
};
