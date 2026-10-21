// State of the window this UI runs in: pinned (1.51) and maximized. V107-R46 A pop-out's
// frame maximizes too (it was fullscreen, V104-03): the taskbar stays visible, and Windows
// itself restores the window under the cursor when its header is dragged.

import { getCurrentWindow } from '@tauri-apps/api/window';

import { windowsIpc } from '../ipc/windows';
import { errorText } from './errors';
import { toast } from './toast.svelte';

class CurrentWindow {
  pinned = $state(false);
  maximized = $state(false);
  isMain = $state(false);
  /** A title the panel picked itself (whispers: the open conversation), over the route's. */
  title = $state<string | null>(null);

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
      toast.error(errorText(e));
    }
  }

  minimize = () => getCurrentWindow().minimize();
  toggleMaximize = () => getCurrentWindow().toggleMaximize();
  close = () => getCurrentWindow().close();

  /** Retitles only this window, in the frame and the OS. */
  setTitle = async (title: string): Promise<void> => {
    this.title = title;
    try {
      await getCurrentWindow().setTitle(title);
    } catch {
      // The frame shows it anyway.
    }
  };
}

export const currentWindow = new CurrentWindow();
