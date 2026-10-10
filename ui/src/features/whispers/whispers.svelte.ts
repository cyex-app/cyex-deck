// Whispers (1.26) in the UI: the thread list Rust keeps, and opening the whispers view
// from anywhere (a name's menu, an inline line, a toast, `/w`).

import type { WhisperState } from '../../ipc/bindings/WhisperState';
import { whispersIpc } from '../../ipc/whispers';
import { currentWindow } from '../../lib/window.svelte';
import { popOut } from '../../app/panels';

const EMPTY: WhisperState = {
  account: null,
  history: false,
  threads: [],
  more: false,
  loading: false,
  blockStrangers: false,
  unread: 0,
};

class WhispersStore {
  state = $state.raw<WhisperState>(EMPTY);
  /** The person the main window should show next, set by `show`. */
  wanted = $state<string | null>(null);
  /** Goes up on every `show`, so the main window switches to the view. */
  asks = $state(0);
  private started = false;

  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await whispersIpc.onChanged((s) => (this.state = s));
    this.state = await whispersIpc.state();
  }

  /** Opens the whispers view, at `login`'s conversation when given. Pop-outs open their own. */
  show(login: string | null = null): void {
    if (currentWindow.isMain) {
      this.wanted = login;
      this.asks++;
    } else {
      void popOut(login ? `whispers/${login}` : 'whispers');
    }
  }
}

export const whispers = new WhispersStore();
