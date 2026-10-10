// C23 Animated 7TV profile pictures, by login. Asked for once per run; the still frame
// shows while the window isn't focused (E2).

import { SvelteMap } from 'svelte/reactivity';

import type { Avatar } from '../../ipc/bindings/Avatar';
import { emotesIpc } from '../../ipc/emotes';
import { focus } from '../../lib/focus.svelte';

class Avatars {
  private map = new SvelteMap<string, Avatar>();
  /** Plain, so asking during a render doesn't write state. */
  private asked = new Set<string>();

  /** The 7TV picture's URL, or `null` to use Twitch's. */
  of(login: string | null | undefined): string | null {
    if (!login) return null;
    const key = login.toLowerCase();
    if (!this.asked.has(key)) {
      this.asked.add(key);
      emotesIpc
        .avatar(key)
        .then((a) => a && this.map.set(key, a))
        .catch(() => {});
    }
    const a = this.map.get(key);
    return a ? (focus.still ? a.still : a.url) : null;
  }
}

export const avatars = new Avatars();
