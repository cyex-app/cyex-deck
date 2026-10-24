// V108-04 Which accounts are signed in on Twitch's pages in the app (wallet, subscriptions,
// Bits, payment methods, clip editor), and "Connect Twitch pages".

import { subsIpc } from '../../ipc/subs';
import { errorText } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';
import { alsoSevenTv } from './alsoSevenTv.svelte';

class TwitchPages {
  connected = $state<string[]>([]);
  /** The account whose login is open right now. */
  connecting = $state<string | null>(null);
  private started = false;

  /** Loads once and follows changes. */
  watch(): void {
    if (this.started) return;
    this.started = true;
    subsIpc.connected().then((ids) => (this.connected = ids)).catch(() => {});
    void subsIpc.onConnected((ids) => (this.connected = ids));
  }

  has(userId: string): boolean {
    return this.connected.includes(userId);
  }

  async connect(userId: string): Promise<void> {
    this.connecting = userId;
    const before = alsoSevenTv.snapshot(this.connected);
    try {
      if (await subsIpc.connect(userId)) {
        toast.success('Twitch pages connected.');
        alsoSevenTv.added('pages', userId, before);
      }
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      if (this.connecting === userId) this.connecting = null;
    }
  }
}

export const twitchPages = new TwitchPages();
