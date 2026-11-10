// V110-A30 Messages kept: the hint line, and a warning from about 3.000 up (every kept line is live DOM).
import { formatNumber } from '../../lib/format.ts';

export const SCROLLBACK_WARN = 3000;

export function scrollbackHint(n: number): string {
  if (n >= SCROLLBACK_WARN) return `${formatNumber(n)} lines per chat use a lot of memory.`;
  return 'Per chat. Older ones drop off the top; more uses more memory.';
}
