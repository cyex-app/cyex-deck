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

  async load(): Promise<void> {
    try {
      this.info = await chatxIpc.cheerInfo(this.view());
      this.error = '';
    } catch (e) {
      this.error = errorText(e);
    }
  }

  /** After a send: what Twitch said is left. */
  setBalance(balance: number | null): void {
    if (this.info && balance != null) this.info = { ...this.info, balance };
  }
}
