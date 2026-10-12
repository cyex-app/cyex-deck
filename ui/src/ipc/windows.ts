import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { WindowInfo } from './bindings/WindowInfo';

export const windowsIpc = {
  info: () => invoke<WindowInfo>('window_info'),
  setPinned: (pinned: boolean) => invoke<boolean>('window_set_pinned', { pinned }),
  setTitle: (title: string) => invoke<void>('window_set_title', { title }),
  openPanel: (route: string, title: string, width?: number, height?: number) =>
    invoke<void>('window_open_panel', { route, title, width, height }),
  /** 3.14 Closing a pop-out docks its panel back. */
  closePanel: (route: string) => invoke<void>('window_close_panel', { route }),
  /** Routes of the open pop-outs. */
  panels: () => invoke<string[]>('window_panels'),
  onPanels: (handler: (routes: string[]) => void) => listen<string[]>('panels:changed', (e) => handler(e.payload)),
  saveLayout: (name: string) => invoke<void>('layout_save', { name }),
  applyLayout: (name: string) => invoke<void>('layout_apply', { name }),
  deleteLayout: (name: string) => invoke<void>('layout_delete', { name }),
};
