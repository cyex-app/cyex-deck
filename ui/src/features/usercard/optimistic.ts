// V107-R45 The ban state the card shows at once after an action, until the mod logs catch up.

import type { BanStatus } from '../../ipc/bindings/BanStatus';

/** What an action just run does to their standing: a status, `null` when it lifts one,
 *  `undefined` when it doesn't touch it. `args` is what follows the command name. */
export function afterAction(name: string, args: string, now: number, by: string | null): BanStatus | null | undefined {
  if (name === 'ban') return { until: null, by, reason: args.split(/\s+/).slice(1).join(' ') };
  if (name === 'unban' || name === 'untimeout') return null;
  if (name === 'timeout') {
    const [, secs, ...why] = args.split(/\s+/);
    const n = Number(secs);
    if (!Number.isFinite(n) || n <= 0) return undefined;
    return { until: new Date(now + n * 1000).toISOString(), by, reason: why.join(' ') };
  }
  return undefined;
}

/** How long a local state is trusted over a log that disagrees (the logs lag). */
export const LOCAL_MS = 8000;

/** The state to show: the local one while the log hasn't caught up, then the log's. */
export function shown(local: { ban: BanStatus | null; at: number } | null, logged: BanStatus | null | undefined, now: number) {
  if (!local) return logged;
  if (logged === undefined) return local.ban;
  const agrees = !!logged === !!local.ban;
  return agrees || now - local.at > LOCAL_MS ? logged : local.ban;
}
