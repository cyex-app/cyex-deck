import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { Accounts } from './bindings/Accounts';
import type { Appearance } from './bindings/Appearance';
import type { Browse } from './bindings/Browse';
import type { Chat } from './bindings/Chat';
import type { Combos } from './bindings/Combos';
import type { Commands } from './bindings/Commands';
import type { Emotes } from './bindings/Emotes';
import type { General } from './bindings/General';
import type { HighlightImport } from './bindings/HighlightImport';
import type { HighlightPreview } from './bindings/HighlightPreview';
import type { Highlights } from './bindings/Highlights';
import type { Hotkeys } from './bindings/Hotkeys';
import type { ImportReport } from './bindings/ImportReport';
import type { MenuLayouts } from './bindings/MenuLayouts';
import type { Miner } from './bindings/Miner';
import type { Modview } from './bindings/Modview';
import type { Notifications } from './bindings/Notifications';
import type { Partition } from './bindings/Partition';
import type { Player } from './bindings/Player';
import type { Plugins } from './bindings/Plugins';
import type { StreamManager } from './bindings/StreamManager';
import type { Tabs } from './bindings/Tabs';
import type { Uploader } from './bindings/Uploader';
import type { Windows } from './bindings/Windows';

/** Every partition with its typed shape. */
export interface SettingsValues {
  general: General;
  appearance: Appearance;
  chat: Chat;
  player: Player;
  highlights: Highlights;
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
  uploader: Uploader;
  combos: Combos;
  menus: MenuLayouts;
}

/** JSON merge patch shape: nested objects merge, arrays replace, `null` resets to default. */
export type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends readonly unknown[]
    ? T[K] | null
    : T[K] extends object
      ? DeepPartial<T[K]> | null
      : T[K] | null;
};

/** V110-U55 `lastOk` in seconds since 1970. */
export interface BackupLine {
  lastOk: number | null;
  error: string | null;
}

/** V110-A43 What an import did: the parts it replaced, and lines to show ("Kept the first 500 commands."). */
export interface Imported {
  partitions: Partition[];
  notes: string[];
}

/** V110-A-idea What a file or share code brings, shown before it is applied. */
export interface ImportReview {
  partitions: Partition[];
  /** "12 commands", "3 sound files"… */
  lines: string[];
}

/** V110-A44 A settings file a start set aside as broken. `name` is what `restoreBroken` takes. */
export interface BrokenFile {
  name: string;
  partition: Partition;
  /** yyyy/mm/dd HH:MM */
  when: string;
  size: number;
}

export const settingsIpc = {
  all: () => invoke<SettingsValues>('settings_all'),
  patch: <P extends Partition>(partition: P, patch: DeepPartial<SettingsValues[P]>) =>
    invoke<SettingsValues[P]>('settings_patch', { partition, patch }),
  pruneBlank: () => invoke<Partial<SettingsValues>>('settings_prune_blank'),
  /** V105-11 The image uploader's headers (kept in the credential store, not in settings). */
  uploaderHeaders: () => invoke<string>('uploader_headers'),
  reset: <P extends Partition>(partition: P) => invoke<SettingsValues[P]>('settings_reset', { partition }),
  exportFile: (partitions: Partition[]) => invoke<boolean>('settings_export_file', { partitions }),
  importFile: () => invoke<Imported>('settings_import_file'),
  /** V1041-02 Opens a file picker; `null` when cancelled. */
  chatterinoPreview: () => invoke<ImportReport | null>('settings_chatterino_preview'),
  /** V1041-02 Adds the previewed tabs, with the stream too if `player`. */
  chatterinoImport: (player: boolean) => invoke<ImportReport>('settings_chatterino_import', { player }),
  /** V105-16 Opens a file picker for Chatterino's settings.json; `null` when cancelled. */
  highlightsPreview: () => invoke<HighlightPreview | null>('highlights_chatterino_preview'),
  /** V105-16 Adds the picked highlights; duplicates keep yours or take Chatterino's. */
  highlightsImport: (keepMine: boolean) => invoke<HighlightImport>('highlights_chatterino_import', { keepMine }),
  shareCode: (partitions: Partition[]) => invoke<string>('settings_share_code', { partitions }),
  importCode: (code: string) => invoke<Imported>('settings_import_code', { code }),
  /** V110-A-idea Picks a file and tells what it brings; nothing changes until `applyImport`. `null` when cancelled. */
  peekFile: () => invoke<ImportReview | null>('settings_import_peek_file'),
  peekCode: (code: string) => invoke<ImportReview>('settings_import_peek_code', { code }),
  applyImport: () => invoke<Imported>('settings_import_apply'),
  cancelImport: () => invoke<void>('settings_import_cancel'),
  backups: () => invoke<string[]>('settings_backups'),
  /** Saves the current settings on disk (no sign-ins); returns the folder name. */
  snapshotNow: () => invoke<string | null>('settings_snapshot_now'),
  /** V110-U55 When the last daily copy was made (seconds since 1970) and why the latest try failed. */
  backupStatus: () => invoke<BackupLine>('settings_backup_status'),
  /** V110-U55 Tries today's copy now. */
  backupNow: () => invoke<BackupLine>('settings_backup_now'),
  restoreBackup: (date: string) => invoke<Partition[]>('settings_restore_backup', { date }),
  /** V110-A44 Files set aside as broken, newest first. */
  brokenFiles: () => invoke<BrokenFile[]>('settings_broken_files'),
  /** V110-A44 Puts one back by its listed name. */
  restoreBroken: (name: string) => invoke<Partition>('settings_restore_broken', { name }),
  openFolder: () => invoke<void>('settings_open_folder'),
  /** V110-N6 Reads the files that couldn't be opened at start again. */
  retryBlocked: () => invoke<{ recovered: Partition[]; blocked: Partition[] }>('settings_retry_blocked'),
  /** V107-N10a Files that went back to default, and files that came back from a backup (day yyyy/mm/dd when all share one). */
  takeLoadReport: () =>
    invoke<{
      reset: Partition[];
      restored: Partition[];
      restoredFrom: string | null;
      blocked: Partition[];
      /** V110-A44 Tabs kept from a tabs file that was cut short. */
      rescuedTabs: number | null;
    }>(
      'settings_take_load_report',
    ),
  /** V110-F82 A partition has failed to save several times in a row. */
  onSaveFailing: (handler: (partitions: Partition[]) => void) =>
    listen<Partition[]>('settings:save-failing', (e) => handler(e.payload)),
  onChanged: (handler: (partition: Partition, value: unknown) => void) =>
    listen<{ partition: Partition; value: unknown }>('settings:changed', (e) =>
      handler(e.payload.partition, e.payload.value),
    ),
};
