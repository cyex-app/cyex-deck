// V107-R23, V107-H1 The usercard follows one user's lines in a room that shows somewhere,
// without a copy of the whole room: it keeps only the ids of their lines (capped) and says
// when an event touched one, so the card reads their messages again.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { RoomEvent } from '../../ipc/bindings/RoomEvent';

/** Their line ids kept; older ones drop off. */
export const KEEP = 200;

const theirs = (m: ChatMessage, userId: string) => !m.notice && m.sender?.id === userId;

/** Their line ids in `list`, the newest `KEEP`. */
export function theirIds(list: readonly ChatMessage[], userId: string): string[] {
  return list.filter((m) => theirs(m, userId)).map((m) => m.id).slice(-KEEP);
}

/** Whether `events` touched a line of `userId`; `ids` (their lines so far) is kept up to
 *  date and capped. */
export function touches(events: readonly RoomEvent[], userId: string, ids: string[]): boolean {
  let hit = false;
  const add = (m: ChatMessage) => {
    if (!theirs(m, userId) || ids.includes(m.id)) return;
    ids.push(m.id);
    if (ids.length > KEEP) ids.splice(0, ids.length - KEEP);
    hit = true;
  };
  for (const e of events) {
    switch (e.type) {
      case 'message':
        add(e.message);
        break;
      case 'update':
        if (theirs(e.message, userId) || ids.includes(e.message.id)) hit = true;
        add(e.message);
        break;
      case 'delete':
      case 'remove':
        if (ids.includes(e.id)) hit = true;
        break;
      case 'clearUser':
        if (e.userId === userId) hit = true;
        break;
      case 'history':
      case 'gap':
        for (const m of e.messages) add(m);
        break;
      case 'cleared':
      case 'wipe':
        if (ids.length) hit = true;
        break;
    }
  }
  return hit;
}
