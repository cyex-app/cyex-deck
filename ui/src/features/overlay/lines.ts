// V110-B4 What the overlay draws: chat messages and the event lines a viewer sees. Info and
// error lines (recording notices, joins, command answers, room modes) are the app's own and stay out.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';

type Kind = NonNullable<ChatMessage['notice']>['kind'];

const APP_KINDS: ReadonlySet<Kind> = new Set<Kind>(['info', 'error']);

export function overlayShows(m: Pick<ChatMessage, 'notice'>): boolean {
  return !m.notice || !APP_KINDS.has(m.notice.kind);
}
