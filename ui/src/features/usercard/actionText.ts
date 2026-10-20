// V101-05 Twitch's mod-log wording, its length ("for 600 seconds") written as chat writes it ("for 10m").
// Only the first one: a reason after it stays as typed.
import { formatDuration } from '../../lib/format.ts';

export function actionText(text: string): string {
  return text.replace(/\bfor (\d+) seconds?\b/, (_, n: string) => `for ${formatDuration(Number(n))}`);
}
