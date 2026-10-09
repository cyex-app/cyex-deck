// State of the window this UI runs in: pinned (1.51) and maximized.

import { getCurrentWindow } from '@tauri-apps/api/window';

import { windowsIpc } from '../ipc/windows';
import { toast } from './toast.svelte';

class CurrentWindow {
  pinned = $state(false);
  maximized = $state(false);
  isMain = $state(false);

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
}

export const currentWindow = new CurrentWindow();
