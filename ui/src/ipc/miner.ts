// The points miner (08): status, log (W4), stats (1.32), adding a channel (1.34) and
// the two mined first (1.44). The list itself is edited as the `miner` settings.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { BalanceUpdate } from './bindings/BalanceUpdate';
import type { LogLine } from './bindings/LogLine';
import type { MinerStatus } from './bindings/MinerStatus';
import type { Stats } from './bindings/Stats';
import type { StatsChannel } from './bindings/StatsChannel';
import type { TabChannel } from './bindings/TabChannel';

export const minerIpc = {
  status: () => invoke<MinerStatus>('miner_status'),
  log: () => invoke<LogLine[]>('miner_log'),
  /** V106-27 Empties the log in every window. */
  clearLog: () => invoke<void>('miner_log_clear'),
  statsChannels: () => invoke<StatsChannel[]>('miner_stats_channels'),
  /** One channel's stats, or every channel's with `channel` null. */
  stats: (channel: string | null) => invoke<Stats>('miner_stats', { channel }),
  clearStats: (channel: string | null) => invoke<void>('miner_stats_clear', { channel }),
  /** 1.34 False if it was in the list already. */
  add: (channel: TabChannel) => invoke<boolean>('miner_add', { channel }),
  /** 1.44 The name of the channel that made room, if one did. */
  first: (channel: TabChannel, on: boolean) => invoke<string | null>('miner_first', { channel, on }),
  onStatus: (handler: (s: MinerStatus) => void) => listen<MinerStatus>('miner:status', (e) => handler(e.payload)),
  onLog: (handler: (l: LogLine) => void) => listen<LogLine>('miner:log', (e) => handler(e.payload)),
  onLogCleared: (handler: () => void) => listen('miner:log:cleared', () => handler()),
  /** A points balance changed, for the chat's points button. */
  onBalance: (handler: (b: BalanceUpdate) => void) =>
    listen<BalanceUpdate>('points:balance', (e) => handler(e.payload)),
};
