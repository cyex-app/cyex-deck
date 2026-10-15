import { invoke } from '@tauri-apps/api/core';

export const systemIpc = {
  /** Opens a twitch.tv link in the default browser. Other hosts are refused. */
  openLink: (url: string) => invoke<void>('system_open_link', { url }),
  /** Opens a link someone posted in chat. Only http and https. */
  openChatLink: (url: string) => invoke<void>('system_open_chat_link', { url }),
  /** V105-11 Opens a link or delete link from an "Uploaded" line. Only ones we recorded. */
  /** V1051-03 Empties the emote, badge and paint picture cache. */
  clearPictureCache: () => invoke<void>('system_clear_picture_cache'),
  openUpload: (url: string) => invoke<void>('upload_open_link', { url }),
};
