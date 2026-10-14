// V1041-02 Text for the Chatterino import: how much goes in, and what stays out and why.

import type { ImportReport } from '../../../ipc/bindings/ImportReport';
import type { SplitKind } from '../../../ipc/bindings/SplitKind';
import { formatNumber } from '../../../lib/format.ts';

const KINDS: Record<SplitKind, string> = {
  mentions: 'Mentions',
  whispers: 'Whispers',
  live: 'Live',
  watching: 'Watching',
  automod: 'AutoMod',
  irc: 'IRC',
  kick: 'Kick',
  youtube: 'YouTube',
  tiktok: 'TikTok',
  misc: 'Other',
  empty: 'Empty',
  other: 'Other',
};

const plural = (n: number, word: string) => `${formatNumber(n)} ${word}${n === 1 ? '' : 's'}`;

/** "1 tab, 1 channel", "12 tabs, 1.024 channels". */
export function counts(r: Pick<ImportReport, 'tabs' | 'channels'>): string {
  return `${plural(r.tabs, 'tab')}, ${plural(r.channels, 'channel')}`;
}

export interface Line {
  label: string;
  text: string;
}

/** One line per reason something stays out. Reasons with nothing in them are left out. */
export function leftOut(r: ImportReport): Line[] {
  const lines: Line[] = [];
  const add = (label: string, n: number, text: string) => {
    if (n) lines.push({ label: `${label} (${formatNumber(n)})`, text });
  };
  add('Already open', r.alreadyOpen.length, r.alreadyOpen.join(', '));
  const kinds = new Map<string, number>();
  for (const { kind, count } of r.notChannels) kinds.set(KINDS[kind], (kinds.get(KINDS[kind]) ?? 0) + count);
  const skipped = [...kinds].map(([label, n]) => (n > 1 ? `${label} ×${formatNumber(n)}` : label));
  add('Not channels', r.notChannels.reduce((sum, k) => sum + k.count, 0), skipped.join(', '));
  add('Not found on Twitch', r.unresolved.length, r.unresolved.join(', '));
  return lines;
}
