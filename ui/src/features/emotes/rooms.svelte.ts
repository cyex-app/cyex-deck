// 1.39 FFZ's mod and VIP pictures per channel. Asked for when a chat draws badges, again
// once the channel's emotes are in.

import { SvelteMap } from 'svelte/reactivity';

import type { RoomBadges } from '../../ipc/bindings/RoomBadges';
import { emotesIpc } from '../../ipc/emotes';

class Rooms {
  private badges = new SvelteMap<string, RoomBadges>();
  private asked = new Set<string>();
  private started = false;

  get(login: string): RoomBadges | undefined {
    if (!this.asked.has(login)) {
      this.asked.add(login);
      this.start();
      void this.load(login);
    }
    return this.badges.get(login);
  }

  private async load(login: string) {
    try {
      this.badges.set(login, await emotesIpc.roomBadges(login));
    } catch {
      // Twitch's own badges show instead.
    }
  }

  private start() {
    if (this.started) return;
    this.started = true;
    void emotesIpc.onLoaded((login) => {
      if (this.asked.has(login)) void this.load(login);
    });
  }
}

export const rooms = new Rooms();
