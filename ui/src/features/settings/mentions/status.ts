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

/** V107-H16 A global row with the Mentions tab off and no feed: say so instead of
 * quietly dropping its matches. */
export function goesNowhere(r: { enabled: boolean; pattern: string; show: boolean; route: string }): string | null {
  return r.enabled && r.pattern.trim() && !r.show && !r.route.trim() ? 'Goes nowhere: pick a feed or keep Mentions on.' : null;
}
