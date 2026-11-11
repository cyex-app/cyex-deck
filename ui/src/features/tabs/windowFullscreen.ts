// V110-B7 Who wants the main window fullscreen: a player's own fullscreen (F) and theatre.
// The window follows `wantFullscreen`; leaving theatre puts it back to maximized or normal
// size (tao restores the placement, `restorePlan` checks it did).

import { getCurrentWindow } from '@tauri-apps/api/window';

import { windowsIpc } from '../../ipc/windows';
import { bootPlan, fullscreenWatch, restorePlan, UNSEEN, wantFullscreen, type WindowBefore } from './theatreWindow';

let player = false;
let theatre = false;
let before: WindowBefore | null = null;
/** A2-5 Theatre's taskbar room is to be given back; done in the queue, ahead of leaving fullscreen. */
let roomOff = false;
let queue: Promise<void> = Promise.resolve();

async function apply(): Promise<void> {
  const win = getCurrentWindow();
  if (roomOff) {
    roomOff = false;
    await windowsIpc.taskbarRoom(false).catch(() => {});
  }
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
  // A2-6 Kept while either theatre or a player still wants fullscreen.
  if (!want) before = null;
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
  /** A2-5 Theatre's taskbar room ends: runs in the same queue, before `setFullscreen(false)`. */
  taskbarRoomOff(): void {
    roomOff = true;
    sync();
  },
  /** V110-N5 R2-1 The window is fullscreen and nobody here wants it (a reload while theatre or a
   *  player held it): give the taskbar room back and leave fullscreen, in the queue. Resolves
   *  true when it did. At app start and from Maximize / Restore. */
  async releaseStray(): Promise<boolean> {
    try {
      if (bootPlan(await getCurrentWindow().isFullscreen(), theatre, player) !== 'leave') return false;
    } catch {
      return false;
    }
    roomOff = true;
    sync();
    return true;
  },
  /** Theatre began or ended. */
  theatre(on: boolean): void {
    if (theatre === on) return;
    theatre = on;
    sync();
  },
};
