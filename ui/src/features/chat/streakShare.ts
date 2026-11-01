// V108-N23 The words of the streak row.

/** "5-stream watch streak". Twitch counts streams in a row; the Share button asks the rest. */
export function streakText(value: number, fmt: (n: number) => string = String): string {
  return value > 0 ? `${fmt(value)}-stream watch streak` : 'Watch streak';
}

/** The narrow chat's version: "5-stream streak". */
export function streakShort(value: number, fmt: (n: number) => string = String): string {
  return value > 0 ? `${fmt(value)}-stream streak` : 'Streak';
}

interface Answer {
  milestone: { id: string; category: string; value: number; shareStatus: string } | null;
  channelId: string | null;
}

interface Chat {
  account: string | null;
  login: string;
  channelId: string | null;
}

/** The milestone a read answer may show in this chat: only a shareable watch streak, only
 *  for the account and channel it was asked for, and not one waved off or shared already. */
export function shown<M extends NonNullable<Answer['milestone']>>(
  answer: (Answer & { milestone: M | null }) | null,
  asked: Chat,
  now: Chat,
  hidden: (id: string) => boolean,
): M | null {
  const m = answer?.milestone;
  if (!m || m.category !== 'WATCH_STREAK' || m.shareStatus !== 'CAN_SHARE' || !(m.value > 0)) return null;
  if (asked.account !== now.account || asked.login.toLowerCase() !== now.login.toLowerCase()) return null;
  if (!answer.channelId || (now.channelId && answer.channelId !== now.channelId)) return null;
  return hidden(m.id) ? null : m;
}
