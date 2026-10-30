// V108-N16 One line per channel you mod where they have a record: "2 bans · 1 timeout".

import type { OtherModCount } from '../../ipc/bindings/OtherModCount';
import { formatNumber } from '../../lib/format.ts';

const n = (count: number, word: string) => `${formatNumber(count)} ${word}${count === 1 ? '' : 's'}`;

export function countText(c: Pick<OtherModCount, 'bans' | 'timeouts' | 'warnings'>): string {
  return [
    c.bans ? n(c.bans, 'ban') : '',
    c.timeouts ? n(c.timeouts, 'timeout') : '',
    c.warnings ? n(c.warnings, 'warning') : '',
  ]
    .filter(Boolean)
    .join(' · ');
}

/** Most bans first, then timeouts; ties by name. */
export function sortCounts(list: OtherModCount[]): OtherModCount[] {
  return [...list].sort((a, b) => b.bans - a.bans || b.timeouts - a.timeouts || a.login.localeCompare(b.login));
}
