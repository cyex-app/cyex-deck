// V108-N14 Theatre mode state: the tile whose stream fills the tab, chat beside it, the tab
// bar and side chrome hidden. Not saved. Esc leaves; a player's fullscreen is separate and
// returns to theatre (the layout never changes under it).

import { escLeaves } from './theatre';
import { keyTaken } from '../../lib/escape';

class Theatre {
  tile = $state<string | null>(null);

  enter(tile: string): void {
    this.tile = tile;
  }

  leave(): void {
    this.tile = null;
  }
}

export const theatre = new Theatre();

let fullscreenEnded = 0;
let wasFullscreen = false;

if (typeof document !== 'undefined') {
  document.addEventListener('fullscreenchange', () => {
    const on = Boolean(document.fullscreenElement);
    if (wasFullscreen && !on) fullscreenEnded = performance.now();
    wasFullscreen = on;
  });

  window.addEventListener('keydown', (e) => {
    const leave = escLeaves(
      {
        key: e.key,
        composing: e.isComposing,
        taken: keyTaken(e),
        fullscreen: Boolean(document.fullscreenElement),
        sinceFullscreen: performance.now() - fullscreenEnded,
      },
      theatre.tile,
    );
    if (!leave) return;
    e.preventDefault();
    theatre.leave();
  });
}
