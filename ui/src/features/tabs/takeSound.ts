// V108-N13 A click (or "Sound here") on a stream tile gives it the tab's sound, even when the
// user had muted that tile: it is unmuted on the way (own volume kept).

import type { Tab } from '../../ipc/bindings/Tab';
import type { Tile } from '../../ipc/bindings/Tile';
import { unmutesOnClaim } from './audioFocus';
import { audioFocus } from './audioFocus.svelte';
import { tabs } from './tabs.svelte';

export function takeSound(tab: Tab, tile: Tile, onControl = false): void {
  const silencedByFocus = tabs.focusHushed(tab, tile.id);
  audioFocus.claim(tab.id, tile.id);
  if (unmutesOnClaim({ silencedByFocus, tileMuted: tile.muted, onControl })) {
    void tabs.apply({ op: 'sound', tile: tile.id, volume: tile.volume, muted: false, boost: tile.boost });
  }
}
