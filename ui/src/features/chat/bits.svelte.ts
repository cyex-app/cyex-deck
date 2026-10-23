// V107-B A chat box's cheer state: the channel's rules and your balance, shared by the
// Bits picker and the send.

import type { CheerInfo } from '../../ipc/bindings/CheerInfo';
import { chatxIpc } from '../../ipc/chatx';
import { errorText } from '../../lib/errors';

export class CheerBox {
  info = $state<CheerInfo | null>(null);
  error = $state('');
  private readonly view: () => number;

  constructor(view: () => number) {
    this.view = view;
  }

  /** False when the room changed while the reply was in flight (it is dropped). */
  async load(): Promise<boolean> {
    const view = this.view();
    try {
      const info = await chatxIpc.cheerInfo(view);
      if (view !== this.view()) return false;
      this.info = info;
      this.error = '';
    } catch (e) {
      if (view !== this.view()) return false;
      this.error = errorText(e);
    }
    return true;
  }

  /** After a send: what Twitch said is left. */
  setBalance(balance: number | null): void {
    if (this.info && balance != null) this.info = { ...this.info, balance };
  }
}
