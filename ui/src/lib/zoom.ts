// App zoom (appearance.zoom): steps for the hotkeys. Rust applies the value to the windows.

import { settings } from './settings.svelte';

export const ZOOMS = [100, 125, 150, 175, 200] as const;

function step(by: number): void {
  const now = settings.v.appearance.zoom;
  const at = Math.max(ZOOMS.findIndex((z) => z === now), 0);
  const next = ZOOMS[Math.min(Math.max(at + by, 0), ZOOMS.length - 1)];
  if (next !== now) void settings.patch('appearance', { zoom: next });
}

/** Handlers for the zoom actions; every window gets them through `useHotkeys`. */
export const zoomHotkeys = {
  'zoom.in': () => step(1),
  'zoom.out': () => step(-1),
  'zoom.reset': () => void settings.patch('appearance', { zoom: 100 }),
};
