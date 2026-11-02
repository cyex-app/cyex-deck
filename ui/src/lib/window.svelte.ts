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

/** V107-R35 Controls, links and text you type, where a press never moves the window. */
const NO_DRAG = 'button, a, input, textarea, select, label, summary, [role="button"], [role="tab"], [role="option"], [contenteditable], .no-drag';

/** V107-R35 Cards (usercards, V108-C27 the roles list) move their window from anywhere that
 *  isn't a control; `skip` adds the card's own parts that keep the mouse (a scroll list). */
export function dragCard(e: MouseEvent, skip = ''): void {
  if (e.button !== 0 || e.detail > 1) return;
  const t = e.target as Element | null;
  if (!t || t.closest(skip ? `${NO_DRAG}, ${skip}` : NO_DRAG)) return;
  e.preventDefault();
  void getCurrentWindow().startDragging();
}
