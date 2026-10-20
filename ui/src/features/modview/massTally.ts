// V107-R21 The mass-action panel's outcome line: one short part per nonzero bucket.
import { formatNumber } from '../../lib/format.ts';
import type { Tally } from '../../ipc/bindings/Tally';

export type MassKind = 'ban' | 'timeout' | 'unban';

export function tallyParts(t: Tally, kind: MassKind): string[] {
  const done = kind === 'unban' ? 'unbanned' : kind === 'timeout' ? 'timed out' : 'banned';
  const already = kind !== 'unban' ? 'already banned' : t.already === 1 ? 'wasn’t banned' : 'weren’t banned';
  const parts: [number, string][] = [
    [t.done, done],
    [t.already, already],
    [t.gone, 'not found or suspended'],
    [t.protected, t.protected === 1 ? 'mod or broadcaster' : 'mods or broadcaster'],
    [t.throttled, 'rate limited'],
    [t.other, 'failed'],
  ];
  return parts.filter(([n]) => n > 0).map(([n, label]) => `${formatNumber(n)} ${label}`);
}

/** Tooltip: the codes behind "failed" and how many sends were retries. */
export function tallyDetail(t: Tally): string {
  const out: string[] = [];
  if (t.codes.length) out.push(`Failed: ${t.codes.join(', ')}`);
  if (t.retries) out.push(`${formatNumber(t.retries)} sent again after Twitch slowed down`);
  return out.join('. ');
}
