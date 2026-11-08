// V110-C70 "Copy full message": one line as `[HH:MM] Name: message`.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { NameStyle } from '../../ipc/bindings/NameStyle';
import type { Timestamps } from '../../ipc/bindings/Timestamps';
import type { UserNote } from '../../ipc/bindings/UserNote';
import { formatTime } from '../../lib/format.ts';
import { shownName } from './names.ts';
import { plainText } from './text.ts';

/** `[14:03] Name: text`, `[14:03] Name text` for /me, `[14:03] text` for a line with no sender. */
export function fullMessage(
  m: ChatMessage,
  style: NameStyle,
  stamps: Pick<Timestamps, 'seconds' | 'clock'>,
  note?: UserNote,
): string {
  const time = `[${formatTime(new Date(m.ts), stamps)}]`;
  const text = plainText(m.body) || m.notice?.text || '';
  if (!m.sender) return `${time} ${text}`;
  const name = shownName(m.sender, style, note).name;
  return m.flags.includes('action') ? `${time} ${name} ${text}` : `${time} ${name}: ${text}`;
}
