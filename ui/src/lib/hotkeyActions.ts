// The hotkey actions and their default combos (S2). Pure data, so tests can import it.

export const ACTIONS = {
  'nav.home': { label: 'Go to home', default: 'Ctrl+1' },
  'nav.discover': { label: 'Go to discover', default: 'Ctrl+2' },
  'nav.miner': { label: 'Go to the miner', default: 'Ctrl+3' },
  'nav.streamManager': { label: 'Go to the stream manager', default: 'Ctrl+4' },
  'nav.whispers': { label: 'Go to whispers', default: 'Ctrl+5' },
  'settings.open': { label: 'Open settings', default: 'Ctrl+,' },
  'window.pin': { label: 'Pin window on top', default: 'Ctrl+Shift+P' },
  'tabs.switcher': { label: 'Go to a tab or channel', default: 'Ctrl+K' },
  'tabs.new': { label: 'Add a tab', default: 'Ctrl+T' },
  'tabs.close': { label: 'Close the tab', default: 'Ctrl+W' },
  'tabs.reopen': { label: 'Reopen closed tab', default: 'Ctrl+Shift+T' },
  'tabs.next': { label: 'Next tab', default: 'Ctrl+Tab' },
  'tabs.prev': { label: 'Previous tab', default: 'Ctrl+Shift+Tab' },
  'tabs.onlyLive': { label: 'Only live tabs on or off', default: 'Ctrl+Shift+L' },
  'player.playPause': { label: 'Play or pause', default: 'Space' },
  'player.mute': { label: 'Mute', default: 'M' },
  'player.fullscreen': { label: 'Fullscreen', default: 'F' },
  'player.back': { label: 'Go back', default: 'ArrowLeft' },
  'player.forward': { label: 'Go forward', default: 'ArrowRight' },
  'player.live': { label: 'Go to live', default: 'Shift+ArrowRight' },
  'player.volumeUp': { label: 'Volume up', default: 'ArrowUp' },
  'player.volumeDown': { label: 'Volume down', default: 'ArrowDown' },
  'player.back2': { label: 'Go back, second key', default: 'J' },
  'player.playPause2': { label: 'Play or pause, second key', default: 'K' },
  'player.forward2': { label: 'Go forward, second key', default: 'L' },
  'chat.search': { label: 'Search this chat', default: 'Ctrl+F' },
  'chat.searchAll': { label: 'Search every chat', default: 'Ctrl+Shift+F' },
  'chat.similar': { label: 'Hide or show near-duplicates', default: 'Ctrl+H' },
  'zoom.in': { label: 'Zoom in', default: 'Ctrl+=' },
  'zoom.out': { label: 'Zoom out', default: 'Ctrl+-' },
  'zoom.reset': { label: 'Reset zoom', default: 'Ctrl+0' },
  'global.show': { label: 'Show or hide cyex deck', default: '', global: true },
  'global.streamer': { label: 'Streamer mode on or off', default: '', global: true },
  'global.clickThrough': { label: 'Overlay click-through on or off', default: '', global: true },
} as const satisfies Record<string, { label: string; default: string; global?: boolean }>;

/** S2 An action's combo: its override, else its default. Empty = unbound. A default that
 * another action was rebound to stays with that action, so a new default (V106-06) never
 * takes a combo the user already uses. */
export function resolveBinding(id: keyof typeof ACTIONS, overrides: Readonly<Record<string, string | null>> = {}): string {
  const own = overrides[id];
  if (own != null) return own;
  const fallback: string = ACTIONS[id].default;
  if (!fallback) return '';
  const taken = Object.entries(overrides).some(([other, combo]) => other !== id && combo === fallback);
  return taken ? '' : fallback;
}

/** V107-N6a The same key on other layouts: Ctrl++ (numpad, German "+") and Ctrl+Shift++
 * (US "+") count as Ctrl+= when nothing is bound to them, like a browser's zoom in. */
const ALIASES: Readonly<Record<string, string>> = { 'Ctrl++': 'Ctrl+=', 'Ctrl+Shift++': 'Ctrl+=' };

export function comboAlias(combo: string): string | undefined {
  return ALIASES[combo];
}
