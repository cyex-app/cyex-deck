import { invoke } from '@tauri-apps/api/core';

import type { WindowInfo } from './bindings/WindowInfo';

export const windowsIpc = {
  info: () => invoke<WindowInfo>('window_info'),
  setPinned: (pinned: boolean) => invoke<boolean>('window_set_pinned', { pinned }),
  openPanel: (route: string, title: string, width?: number, height?: number) =>
    invoke<void>('window_open_panel', { route, title, width, height }),
  saveLayout: (name: string) => invoke<void>('layout_save', { name }),
  applyLayout: (name: string) => invoke<void>('layout_apply', { name }),
  deleteLayout: (name: string) => invoke<void>('layout_delete', { name }),
};
