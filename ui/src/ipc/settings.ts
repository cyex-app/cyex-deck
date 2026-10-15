import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { Accounts } from './bindings/Accounts';
import type { Appearance } from './bindings/Appearance';
import type { Browse } from './bindings/Browse';
import type { Chat } from './bindings/Chat';
import type { Commands } from './bindings/Commands';
import type { Emotes } from './bindings/Emotes';
import type { General } from './bindings/General';
import type { HighlightImport } from './bindings/HighlightImport';
import type { HighlightPreview } from './bindings/HighlightPreview';
import type { Highlights } from './bindings/Highlights';
import type { Hotkeys } from './bindings/Hotkeys';
import type { ImportReport } from './bindings/ImportReport';
import type { Mentions } from './bindings/Mentions';
import type { Miner } from './bindings/Miner';
import type { Modview } from './bindings/Modview';
import type { Notifications } from './bindings/Notifications';
import type { Partition } from './bindings/Partition';
import type { Player } from './bindings/Player';
import type { Plugins } from './bindings/Plugins';
import type { StreamManager } from './bindings/StreamManager';
import type { Tabs } from './bindings/Tabs';
import type { Windows } from './bindings/Windows';

/** Every partition with its typed shape. */
export interface SettingsValues {
  general: General;
  appearance: Appearance;
  chat: Chat;
  player: Player;
  highlights: Highlights;
  mentions: Mentions;
  commands: Commands;
  hotkeys: Hotkeys;
  tabs: Tabs;
  browse: Browse;
  emotes: Emotes;
  accounts: Accounts;
  miner: Miner;
  modview: Modview;
  streammanager: StreamManager;
  notifications: Notifications;
  windows: Windows;
  plugins: Plugins;
}

/** JSON merge patch shape: nested objects merge, arrays replace, `null` resets to default. */
export type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends readonly unknown[]
    ? T[K] | null
    : T[K] extends object
      ? DeepPartial<T[K]> | null
      : T[K] | null;
};

export const settingsIpc = {
  all: () => invoke<SettingsValues>('settings_all'),
  patch: <P extends Partition>(partition: P, patch: DeepPartial<SettingsValues[P]>) =>
    invoke<SettingsValues[P]>('settings_patch', { partition, patch }),
  pruneBlank: () => invoke<Partial<SettingsValues>>('settings_prune_blank'),
  reset: <P extends Partition>(partition: P) => invoke<SettingsValues[P]>('settings_reset', { partition }),
  exportFile: (partitions: Partition[]) => invoke<boolean>('settings_export_file', { partitions }),
  importFile: () => invoke<Partition[]>('settings_import_file'),
  /** V1041-02 Opens a file picker; `null` when cancelled. */
  chatterinoPreview: () => invoke<ImportReport | null>('settings_chatterino_preview'),
  /** V1041-02 Adds the previewed tabs, with the stream too if `player`. */
  chatterinoImport: (player: boolean) => invoke<ImportReport>('settings_chatterino_import', { player }),
  /** V105-16 Opens a file picker for Chatterino's settings.json; `null` when cancelled. */
  highlightsPreview: () => invoke<HighlightPreview | null>('highlights_chatterino_preview'),
  /** V105-16 Adds the picked highlights; duplicates keep yours or take Chatterino's. */
  highlightsImport: (keepMine: boolean) => invoke<HighlightImport>('highlights_chatterino_import', { keepMine }),
  shareCode: (partitions: Partition[]) => invoke<string>('settings_share_code', { partitions }),
  importCode: (code: string) => invoke<Partition[]>('settings_import_code', { code }),
  backups: () => invoke<string[]>('settings_backups'),
  restoreBackup: (date: string) => invoke<Partition[]>('settings_restore_backup', { date }),
  openFolder: () => invoke<void>('settings_open_folder'),
  takeLoadReport: () => invoke<Partition[]>('settings_take_load_report'),
  onChanged: (handler: (partition: Partition, value: unknown) => void) =>
    listen<{ partition: Partition; value: unknown }>('settings:changed', (e) =>
      handler(e.payload.partition, e.payload.value),
    ),
};
