// V109-C01 Reply threads: which lines belong to one, and what to show when the first line
// is no longer in the buffer. Free of Svelte so it runs under the UI tests. A thread is the
// message that started it plus every reply carrying its id as `threadId`.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';

/** The thread's own window: `thread/<channel>/<id of the first message>`. */
export function threadRoute(channel: string, threadId: string): string {
  return `thread/${channel}/${threadId}`;
}

export function parseThreadRoute(route: string): { channel: string; threadId: string } {
  const [, channel = '', threadId = ''] = route.split('/');
  return { channel, threadId };
}

/** Whether `m` is the first message of thread `id` or a reply in it. */
export function inThread(m: ChatMessage, id: string): boolean {
  return !m.notice && (m.id === id || m.reply?.threadId === id);
}

/** The thread's lines, oldest first (the buffer is already in order; a repeated id is dropped). */
export function collect(list: readonly ChatMessage[], id: string): ChatMessage[] {
  const seen = new Set<string>();
  const out: ChatMessage[] = [];
  for (const m of list) {
    if (!inThread(m, id) || seen.has(m.id)) continue;
    seen.add(m.id);
    out.push(m);
  }
  return out;
}

export interface Orphan {
  login: string;
  display: string;
  body: string;
}

/** The first message when it has left the buffer: what the reply tags carry about it (a
 *  reply straight to it names its sender and text). `null` when it is here, or when no
 *  reply here answers it directly. */
export function orphanRoot(list: readonly ChatMessage[], id: string): Orphan | null {
  if (list.some((m) => m.id === id && !m.notice)) return null;
  for (const m of list) {
    const r = m.reply;
    if (r && r.parentId === id && r.threadId === id) return { login: r.parentLogin, display: r.parentDisplay, body: r.parentBody };
  }
  return null;
}
