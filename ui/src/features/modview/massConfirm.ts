// V110-A2 Mass ban / mass time out with more than one name asks first, like Unban does:
// "Ban 480?" in the bubble. One name goes straight through.
import { formatNumber } from '../../lib/format.ts';

/** The bubble's text for a ban or time out of `count` logins, or null when it needs no ask. */
export function massConfirm(count: number, timeout: boolean): string | null {
  if (!(count > 1)) return null;
  return `${timeout ? 'Time out' : 'Ban'} ${formatNumber(count)}?`;
}
