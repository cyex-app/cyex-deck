import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { StreamerState } from './bindings/StreamerState';

export const modesIpc = {
  /** U4 */
  streamer: () => invoke<StreamerState>('streamer_state'),
  toggleStreamer: () => invoke<StreamerState>('streamer_toggle'),
  onStreamer: (cb: (s: StreamerState) => void) => listen<StreamerState>('streamer:changed', (e) => cb(e.payload)),
  /** U7 For the calling overlay window. */
  clickThrough: () => invoke<boolean>('overlay_click_through'),
  setClickThrough: (on: boolean) => invoke<void>('overlay_set_click_through', { on }),
  onClickThrough: (cb: (on: boolean) => void) => listen<boolean>('overlay:clickThrough', (e) => cb(e.payload)),
  /** S2 Global combos another app already holds. */
  hotkeysFailed: () => invoke<string[]>('hotkeys_failed'),
  onHotkeysFailed: (cb: (combos: string[]) => void) => listen<string[]>('hotkeys:failed', (e) => cb(e.payload)),
};
