// V108-C10 A Shared Chat line carries the source channel's badges (`source-badges`) and the
// sender's own (`badges`). Global sets (Prime, Turbo, partner, staff...) are in both, so
// drawn side by side they showed twice. One list: the source's first, then the sender's
// sets the source list doesn't have. Each ref says whose catalog draws it.

export interface BadgeRef {
  set: string;
  version: string;
  info?: string;
  /** The source room id when this badge comes from the source channel's list. */
  room?: string;
}

export function lineBadges(
  source: { roomId: string; badges: BadgeRef[] } | undefined | null,
  sender: BadgeRef[],
): BadgeRef[] {
  if (!source) return sender;
  const seen = new Set<string>();
  const out: BadgeRef[] = [];
  for (const b of source.badges) {
    if (seen.has(b.set)) continue;
    seen.add(b.set);
    out.push({ ...b, room: source.roomId });
  }
  for (const b of sender) {
    if (seen.has(b.set)) continue;
    seen.add(b.set);
    out.push(b);
  }
  return out;
}
