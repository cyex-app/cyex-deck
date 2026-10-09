import { invoke } from '@tauri-apps/api/core';

export const systemIpc = {
  /** Opens a twitch.tv link in the default browser. Other hosts are refused. */
  openLink: (url: string) => invoke<void>('system_open_link', { url }),
};
