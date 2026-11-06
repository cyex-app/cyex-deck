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

/** Who the thread's message box answers by default: its first message, or the newest line
 *  when that has left the buffer (Rust needs the line to answer it; the reply keeps the
 *  thread's id from it). `null`: nothing to answer. */
export function defaultReply(lines: readonly ChatMessage[], id: string): ChatMessage | null {
  const usable = (m: ChatMessage) => !m.notice && !!m.sender;
  const first = lines.find((m) => m.id === id && usable(m));
  if (first) return first;
  for (let i = lines.length - 1; i >= 0; i--) if (usable(lines[i])) return lines[i];
  return null;
}
