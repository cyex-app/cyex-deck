// V104-02 The firehose connection in a few words; a sound file's name.

import type { GlobalStatus } from '../../../ipc/bindings/GlobalStatus';

export function statusText(s: GlobalStatus, hasMentions: boolean): string {
  switch (s.state) {
    case 'connecting':
      return 'Connecting…';
    case 'connected':
      return 'Connected';
    case 'offline':
      return `Offline. Trying again in ${s.retrySecs}s.`;
    default:
      return hasMentions ? 'Off' : 'Add a mention to start.';
  }
}

/** "C:\Sounds\ping.wav" → "ping.wav". */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** V107-H16 A global row that lands nowhere: say so instead of quietly dropping its matches.
 *  V108-GM `placed`: global mentions have a tab or a feed, so the row's tab choice lands. */
export function goesNowhere(r: { enabled: boolean; pattern: string; show: boolean; route: string }, placed = true): string | null {
  if (!r.enabled || !r.pattern.trim() || r.route.trim() || (r.show && placed)) return null;
  return placed ? 'Goes nowhere: pick a feed or keep Global tab on.' : 'Goes nowhere: pick a feed for it or for all global mentions.';
}
