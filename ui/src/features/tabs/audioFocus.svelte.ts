// V108-N13 Which tile of a tab was last picked to have the sound (not saved: every tab starts
// with its first stream). Whether it applies is Settings → Appearance; `tabs.tileMuted` reads it.

import { settings } from '../../lib/settings.svelte';
import { type FocusInput, hushedByFocus } from './audioFocus';
import { theatre } from './theatre.svelte';

class AudioFocus {
  /** Tab id → the tile picked there. */
  claims = $state.raw<Map<string, string>>(new Map());

  claim(tab: string, tile: string): void {
    if (this.claims.get(tab) === tile) return;
    this.claims = new Map(this.claims).set(tab, tile);
  }

  forget(tab: string): void {
    if (!this.claims.has(tab)) return;
    const next = new Map(this.claims);
    next.delete(tab);
    this.claims = next;
  }

  input(tab: string, players: readonly string[]): FocusInput {
    return {
      players,
      claimed: this.claims.get(tab) ?? null,
      enabled: settings.v.appearance.audioFocus,
      theatre: theatre.tile,
    };
  }

  /** The focus silences `tile` (one of `players`, the stream tiles of `tab`). */
  hushed(tab: string, tile: string, players: readonly string[]): boolean {
    return hushedByFocus(tile, this.input(tab, players));
  }
}

export const audioFocus = new AudioFocus();
