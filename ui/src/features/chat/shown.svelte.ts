// V103-22 Where each channel's regular chat shows, for this window: one source for the mod
// view's and the stream manager's own chat panels. Rust sends it as views open and close
// (`chat:shown`, once they've been still a moment); while another window is still loading
// the last settled picture stays, so nothing flickers while a saved layout comes up.

import { getCurrentWindow } from '@tauri-apps/api/window';

import type { ChatShown } from '../../ipc/bindings/ChatShown';
import { chatIpc } from '../../ipc/chat';
import { elsewhere, settled } from './follow';

class ShownStore {
  /** As of the last moment no other window was loading; `null` until then. */
  private held = $state.raw<ChatShown['chats'] | null>(null);
  private started = false;
  private own = '';

  /** Listens from the first view on; later calls do nothing. */
  start(): void {
    if (this.started) return;
    this.started = true;
    this.own = getCurrentWindow().label;
    // Listening first, then asking, so a change in between isn't missed.
    let heard = false;
    chatIpc
      .onShown((s) => {
        heard = true;
        this.take(s);
      })
      .then(() => chatIpc.shown())
      .then((s) => !heard && this.take(s))
      .catch(() => {});
  }

  /** `login`'s regular chat shows in another window; `null` until that's known. */
  elsewhere(login: string): boolean | null {
    return this.held ? elsewhere(this.held, login, this.own) : null;
  }

  private take(s: ChatShown): void {
    if (settled(s, this.own)) this.held = s.chats;
  }
}

export const chatShown = new ShownStore();
