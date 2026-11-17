// V111-53 "Show in chat" from a feed tab: the main window goes to the tab (App), and the chat
// in that tile scrolls to the line and lights it (ChatView, MessageList's jump).

import { toast } from '../../lib/toast.svelte';
import type { ChatSpot } from './feedJump';

export interface JumpAsk extends ChatSpot {
  id: string;
  n: number;
}

/** A chat that never shows up (its tab closed meanwhile) drops the ask. */
const WAIT_MS = 4000;

class ChatJump {
  ask = $state.raw<JumpAsk | null>(null);
  private n = 0;

  go(spot: ChatSpot, id: string): void {
    const n = ++this.n;
    this.ask = { ...spot, id, n };
    setTimeout(() => {
      if (this.ask?.n !== n) return;
      this.ask = null;
      toast.info('Couldn’t find that chat.');
    }, WAIT_MS);
  }

  /** The chat of `tile` takes the ask, once. */
  take(tile: string | undefined): JumpAsk | null {
    const a = this.ask;
    if (!a || !tile || a.tile !== tile) return null;
    this.ask = null;
    return a;
  }
}

export const chatJump = new ChatJump();
