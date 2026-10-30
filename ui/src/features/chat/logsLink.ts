// V108-N17 A message's own place on logs.cyex.app, the way its "Copy link" builds it:
// the channel's day (UTC, rustlog's buckets) with a #msg- anchor it loads and scrolls to.
// For lines older than what's loaded, e.g. global mentions.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';

const LOGS = 'https://logs.cyex.app/';

const pad = (n: number) => String(n).padStart(2, '0');

/** The link, or null for lines the logs can't have (local system lines, no time). */
export function logsMessageUrl(channel: string, m: Pick<ChatMessage, 'id' | 'ts' | 'source'>): string | null {
  if (!m.id || m.id.startsWith('local-') || !(m.ts > 0)) return null;
  // Shared chat: the line lives in the channel it was sent in.
  const c = m.source?.roomId ? `id:${m.source.roomId}` : channel;
  if (!c) return null;
  const d = new Date(m.ts);
  const day = `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}`;
  const q = new URLSearchParams({ c, d: day });
  return `${LOGS}?${q}#msg-${encodeURIComponent(m.id)}`;
}
