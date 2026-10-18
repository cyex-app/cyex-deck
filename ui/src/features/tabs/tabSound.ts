// V106-25 What a tab's sound badge shows.

export type TabSoundMode = 'audible' | 'muted' | 'silent';

/** Muted shows while the tab mute is on, whatever plays; else audible while something would be
 * heard. `heldMuted`: V102-15 a tab playing out of sight with every player muted in the player
 * shows the crossed badge too. */
/** The tooltip suffix for a tab's sound state (U2). */
export function soundTip(mode: TabSoundMode, tabMuted: boolean): string {
  if (tabMuted) return ' · muted';
  if (mode === 'audible') return ' · playing';
  return mode === 'muted' ? ' · playing muted' : '';
}

export function tabSoundMode(wouldPlay: boolean, tabMuted: boolean, heldMuted = false): TabSoundMode {
  if (tabMuted) return 'muted';
  if (wouldPlay) return 'audible';
  return heldMuted ? 'muted' : 'silent';
}
