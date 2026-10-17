// V106-27 The miner log's kinds as filter chips and the filter itself.

import type { LogLine } from '../../ipc/bindings/LogLine';

export const CHIPS = [
  { key: 'points', label: 'Points' },
  { key: 'claim', label: 'Chests' },
  { key: 'raid', label: 'Raids' },
  { key: 'bet', label: 'Bets' },
  { key: 'drop', label: 'Drops' },
  { key: 'channel', label: 'Channels' },
  { key: 'error', label: 'Errors' },
] as const;

/** Newest first, without the kinds that are off. */
export function visible(lines: LogLine[], off: ReadonlySet<string>): LogLine[] {
  const out: LogLine[] = [];
  for (let i = lines.length - 1; i >= 0; i--) if (!off.has(lines[i].kind)) out.push(lines[i]);
  return out;
}
