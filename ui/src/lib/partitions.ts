import type { Partition } from '../ipc/bindings/Partition';

/** Display names for settings partitions, in settings-menu order. */
export const PARTITION_LABELS: Record<Partition, string> = {
  general: 'General',
  appearance: 'Appearance',
  chat: 'Chat',
  player: 'Player',
  highlights: 'Highlights',
  commands: 'Commands',
  hotkeys: 'Hotkeys',
  tabs: 'Tabs',
  browse: 'Home and discover',
  emotes: 'Emote favorites',
  accounts: 'Accounts',
  miner: 'Miner',
  modview: 'Mod view',
  streammanager: 'Stream manager',
  notifications: 'Alerts',
  windows: 'Windows and layouts',
  plugins: 'Plugins',
  uploader: 'Image uploads',
  combos: '7TV combos',
  menus: 'Menus',
};

export const ALL_PARTITIONS = Object.keys(PARTITION_LABELS) as Partition[];

/** Partitions that can be exported or shared. Accounts, plugin settings and the image uploader (they can hold
 * keys) stay on this PC. */
export const PORTABLE_PARTITIONS = ALL_PARTITIONS.filter((p) => p !== 'accounts' && p !== 'plugins' && p !== 'uploader');

/** N3-5 Parts in the app's menu order, whatever order the file lists them in. */
export function inMenuOrder(partitions: Partition[]): Partition[] {
  return [...partitions].sort((a, b) => ALL_PARTITIONS.indexOf(a) - ALL_PARTITIONS.indexOf(b));
}

export function partitionList(partitions: Partition[]): string {
  return inMenuOrder(partitions).map((p) => PARTITION_LABELS[p]).join(', ');
}

/** V110-F92 The import toast: an import with automations brings them in switched off. */
export function importedText(partitions: Partition[], notes: string[] = []): string {
  const base = `Imported ${partitionList(partitions)}.`;
  // V110-A43 A line for each list that was cut at its cap, V110-A7 and for a wallpaper that didn't come along.
  const lines = [partitions.includes('commands') ? `${base} Automations are off until you switch them on.` : base, ...notes];
  return lines.join(' ');
}
