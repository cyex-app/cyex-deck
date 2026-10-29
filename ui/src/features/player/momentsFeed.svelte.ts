// V108-N25 The moments of one channel's chat for one player: what the chat found so far,
// then each new batch as it comes (chat:moments). Kept in memory, newest 200.

import type { Moment } from '../../ipc/bindings/Moment';
import { chatxIpc } from '../../ipc/chatx';
import { mergeMoments } from './moments';

export class MomentsFeed {
  list = $state.raw<Moment[]>([]);
  private login = '';
  /** Only the newest follow() counts. */
  private turn = 0;
  private off: (() => void) | undefined;

  /** Starts on `login`'s chat (a tab can change channel in place); does nothing for the same. */
  follow(login: string): void {
    const name = login.toLowerCase();
    if (name === this.login) return;
    this.stop();
    this.login = name;
    this.list = [];
    const turn = ++this.turn;
    void (async () => {
      try {
        const off = await chatxIpc.onMoments((found) => {
          if (turn === this.turn && found.channel.toLowerCase() === name) this.list = mergeMoments(this.list, found.moments);
        });
        if (turn !== this.turn) return off();
        this.off = off;
        const have = await chatxIpc.moments(name);
        if (turn === this.turn) this.list = mergeMoments(this.list, have);
      } catch {
        // No chat in this tab: no moments.
      }
    })();
  }

  destroy(): void {
    this.stop();
    this.login = '';
    this.turn++;
  }

  private stop(): void {
    this.turn++;
    this.off?.();
    this.off = undefined;
  }
}
