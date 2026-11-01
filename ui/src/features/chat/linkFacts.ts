// V108-C25 The words on a link's card: its kind and its facts, in the app's formats
// (dates yyyy/mm/dd, numbers with dots; 3.1, 3.13).

import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
import type { PreviewFact } from '../../ipc/bindings/PreviewFact';
import { formatDate, formatDuration, formatNumber } from '../../lib/format.ts';

const count = (n: number, one: string, many: string) => `${formatNumber(n)} ${n === 1 ? one : many}`;

export function factText(f: PreviewFact): string {
  switch (f.t) {
    case 'views':
      return count(f.n, 'view', 'views');
    case 'viewers':
      return count(f.n, 'viewer', 'viewers');
    case 'length':
      return formatDuration(f.secs);
    case 'date': {
      const d = new Date(f.at);
      return Number.isNaN(d.getTime()) ? '' : formatDate(d);
    }
    case 'game':
      return f.name;
    case 'clippedBy':
      return `Clipped by ${f.name}`;
  }
}

/** Counts, lengths and dates are data: drawn in mono. */
export const numeric = (f: PreviewFact) => f.t !== 'game' && f.t !== 'clippedBy';

/** The small line over the title, e.g. "Twitch clip". */
export function kindText(p: LinkPreview): string {
  switch (p.kind) {
    case 'clip':
      return 'Twitch clip';
    case 'video':
      return 'Twitch video';
    case 'channel':
      return 'Twitch channel';
    case 'web':
      return p.site;
  }
}
