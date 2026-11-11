// 1.13, V105-05 The top-right counter: the global account's open chats, and a warning
// naming the channels when Twitch refused some.

import type { ConnectionCount } from '../../ipc/bindings/ConnectionCount';
import { formatNumber } from '../../lib/format.ts';

export interface CounterView {
  text: string;
  warn: boolean;
  label: string;
  /** What a screen reader says. */
  aria: string;
  hint?: string;
  /** Nothing joined yet: the counter stays out of the bar. */
  hidden: boolean;
}

const SHOWN = 5;
/** V111-N1 */
const LIMIT = 'Twitch caps how many chats one account can join.';
/** Up to five channels, then how many more. */
const listed = (logins: string[]) =>
  logins.slice(0, SHOWN).map((l) => `#${l}`).join(', ') +
  (logins.length > SHOWN ? ` and ${formatNumber(logins.length - SHOWN)} more` : '');

/** `current`: the global account's id. `name` gives an account's display name. */
export function counterView(counts: ConnectionCount[], current: string, name: (id: string) => string): CounterView {
  const own = counts.find((c) => c.userId === current);
  const text = formatNumber(own?.joined ?? 0);
  const refused = counts.filter((c) => c.refused.length);
  const hidden = !own?.joined && !refused.length;
  if (refused.length) {
    return {
      text,
      warn: true,
      hidden,
      label: 'Too many chats joined',
      aria: `Too many chats joined. ${text} open.`,
      hint: `${refused
        .map((c) => `${c.userId === current ? '' : `${name(c.userId)}: `}${listed(c.refused)}`)
        .join('; ')}: not joined. Close a few chats, then open these again.`,
    };
  }
  return {
    text,
    warn: false,
    hidden,
    label: 'Chats you’re in',
    aria: own?.joined === 1 ? '1 chat joined' : `${text} chats joined`,
    hint: counts.length > 1 ? `${LIMIT}
${counts.map((c) => `${name(c.userId)} ${formatNumber(c.joined)}`).join(', ')}` : LIMIT,
  };
}
