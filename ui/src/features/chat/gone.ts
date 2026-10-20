// V107-R34 Why a deleted line went: the small note after its text, and the merge Rust does
// (crates/chat/src/model/message.rs `Gone::merged`) for events that reach a line twice.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { Gone } from '../../ipc/bindings/Gone';
import { formatNumber } from '../../lib/format.ts';

const WORDS: Record<Gone['how'], string> = { deleted: 'deleted', timedOut: 'timed out', banned: 'banned' };

/** "(deleted)", "(timed out 600s by Kuni)"; nothing for a line hidden without Twitch saying why. */
export function goneNote(gone: Gone | undefined): string {
  if (!gone) return '';
  const secs = gone.how === 'timedOut' && gone.secs ? ` ${formatNumber(gone.secs)}s` : '';
  return `(${WORDS[gone.how]}${secs}${gone.by ? ` by ${gone.by}` : ''})`;
}

/** The newer answer wins; the same action told twice (IRC and a mod feed) keeps what either
 *  knew. A feed's length can be a second short; a timeout of another length is a new one. */
export function mergeGone(next: Gone, old: Gone | undefined): Gone {
  const far = next.secs != null && old?.secs != null && Math.abs(next.secs - old.secs) > 5;
  if (!old || old.how !== next.how || far) return next;
  const secs = Math.max(next.secs ?? 0, old.secs ?? 0);
  return { how: next.how, ...((next.by ?? old.by) ? { by: next.by ?? old.by } : {}), ...(secs ? { secs } : {}) };
}

/** `m` marked deleted, and why when known; null when nothing changes. */
export function markGone(m: ChatMessage, gone: Gone | undefined): ChatMessage | null {
  const was = m.flags.includes('deleted');
  const g = gone ? mergeGone(gone, m.gone) : m.gone;
  if (was && g?.how === m.gone?.how && g?.by === m.gone?.by && g?.secs === m.gone?.secs) return null;
  return { ...m, flags: was ? m.flags : [...m.flags, 'deleted'], ...(g ? { gone: g } : {}) };
}
