// 1.46 What a mod view (or one of its panels in its own window, V107-R29) shows for a
// channel: its id and the mod board, live. Make it while a component starts.

import type { Board0 } from '../../ipc/bindings/Board0';
import { modIpc } from '../../ipc/moderation';
import { mods } from './moderation.svelte';

export class ModBoard {
  channelId = $state<string | null>(null);
  board = $state<Board0>({ activity: [], actions: [], held: [], suspicious: [], older: false });
  /** What failed to load, with Retry; `retry` re-runs the loads. */
  problem = $state('');
  retry = $state(0);

  constructor(login: () => string, account: () => string) {
    // The id comes from an open room. V102-26 From a tab with only the stream, that's this
    // view's chat joining: asked again whenever the roles change until it's known.
    $effect(() => {
      void this.retry;
      if (this.channelId) return;
      const l = login();
      void mods.role(account(), l);
      let gone = false;
      modIpc
        .channelId(l)
        .then((id) => {
          if (gone) return;
          this.problem = '';
          if (id && !this.channelId) this.channelId = id;
        })
        .catch(() => !gone && (this.problem = 'Couldn’t load this channel.'));
      return () => (gone = true);
    });

    // Listening first, then the snapshot, so an update in between (Twitch's history coming
    // in, V103-15) isn't missed.
    $effect(() => {
      const id = this.channelId;
      if (!id) return;
      void this.retry;
      let stop: (() => void) | undefined;
      let gone = false;
      modIpc
        .onBoard(id, (b) => (this.board = b))
        .then((u) => {
          if (gone) return u();
          stop = u;
          return modIpc.view(id).then((b) => {
            if (gone) return;
            this.board = b;
            this.problem = '';
          });
        })
        .catch(() => !gone && (this.problem = 'Couldn’t load the mod activity.'));
      return () => {
        gone = true;
        stop?.();
      };
    });
  }
}
