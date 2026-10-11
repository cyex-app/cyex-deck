// 1.46, 1.47, V102-16 Where a channel's mod view shows. The shield on a chat in the main
// window opens it there, as a view next to the tabs; from there it pops out into its own
// window and comes back when that window closes. From a chat in another window (a popped-out
// tab or chat) it opens in a window of its own.

import { popOut } from '../../app/panels';
import { currentWindow } from '../../lib/window.svelte';

export const modviewRoute = (login: string) => `modview/${login}`;

class ModViewPlace {
  /** The channel the main window's mod view is for. */
  login = $state<string | null>(null);
  /** Goes up on every `open`, so the main window switches to the view. */
  asks = $state(0);

  open(login: string): void {
    if (currentWindow.isMain) {
      this.login = login;
      this.asks++;
    } else {
      void popOut(modviewRoute(login), `Mod view · ${login}`);
    }
  }
}

export const modviewPlace = new ModViewPlace();
