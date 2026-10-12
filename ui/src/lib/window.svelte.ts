// State of the window this UI runs in: pinned (1.51), maximized, and fullscreen from a
// pop-out's frame (V104-03).

import { getCurrentWindow } from '@tauri-apps/api/window';

import { windowsIpc } from '../ipc/windows';
import { toast } from './toast.svelte';

class CurrentWindow {
  pinned = $state(false);
  maximized = $state(false);
  isMain = $state(false);
  /** V104-03 The whole window fills the screen, asked for by the frame's button. */
  fullscreen = $state(false);

  async init(): Promise<void> {
    const win = getCurrentWindow();
    const info = await windowsIpc.info();
    this.pinned = info.pinned;
    this.isMain = info.isMain;
    const sync = async () => {
      this.maximized = await win.isMaximized();
    };
    await sync();
    await win.onResized(sync);
  }

  async togglePin(): Promise<void> {
    try {
      this.pinned = await windowsIpc.setPinned(!this.pinned);
    } catch (e) {
      toast.error(String(e));
    }
  }

  minimize = () => getCurrentWindow().minimize();
  toggleMaximize = () => getCurrentWindow().toggleMaximize();
  close = () => getCurrentWindow().close();

  /** V104-03 The window over the whole screen (V103-14 makes it really fill it), or back
   * to the size and place it had. A player's own fullscreen keeps it while this is on. */
  setFullscreen = async (on: boolean): Promise<void> => {
    this.fullscreen = on;
    try {
      await getCurrentWindow().setFullscreen(on || Boolean(document.fullscreenElement));
    } catch (e) {
      this.fullscreen = !on;
      toast.error(String(e));
    }
  };

  toggleFullscreen = () => this.setFullscreen(!this.fullscreen);
}

export const currentWindow = new CurrentWindow();
