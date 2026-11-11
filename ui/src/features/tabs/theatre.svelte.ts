// V108-N14 Theatre mode state: the tile whose stream fills the tab, chat beside it, the tab
// bar and side chrome hidden. Not saved. Esc leaves; a player's fullscreen is separate and
// returns to theatre (the layout never changes under it).

import { escLeaves } from './theatre';
import { keyTaken } from '../../lib/escape';

class Theatre {
  tile = $state<string | null>(null);
  /** V110-B10 The tile's chat is folded away by the player's button; lasts as long as theatre. */
  chatHidden = $state(false);

  enter(tile: string): void {
    // V110-N5 R2-4 A focused text field (the chat box) would keep the first Esc: let go of it.
    if (typeof document !== 'undefined' && document.activeElement?.matches?.('input, textarea, select, [contenteditable]')) (document.activeElement as HTMLElement).blur();
    this.tile = tile;
    this.chatHidden = false;
  }

  leave(): void {
    this.tile = null;
    this.chatHidden = false;
  }

  toggleChat(): void {
    this.chatHidden = !this.chatHidden;
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
    // A text field (the chat box) keeps its Escape.
    if ((e.target as HTMLElement | null)?.closest?.('input, textarea, select, [contenteditable]')) return;
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
