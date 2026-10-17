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
  notifications: 'Notifications',
  windows: 'Windows and layouts',
  plugins: 'Plugins',
  uploader: 'Image uploads',
  combos: '7TV combos',
};

export const ALL_PARTITIONS = Object.keys(PARTITION_LABELS) as Partition[];

/** Partitions that can be exported or shared. Accounts, plugin settings and the image uploader (they can hold
 * keys) stay on this PC. */
export const PORTABLE_PARTITIONS = ALL_PARTITIONS.filter((p) => p !== 'accounts' && p !== 'plugins' && p !== 'uploader');

export function partitionList(partitions: Partition[]): string {
  return partitions.map((p) => PARTITION_LABELS[p]).join(', ');
}
