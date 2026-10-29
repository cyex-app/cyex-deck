// V108-N01, N25 Clips this app just made from a VOD or a moment: the overlay that plays one
// knows Twitch needs a few seconds before it has the files (the player asks again).

const fresh = new Set<string>();

export function markFresh(slug: string): void {
  fresh.add(slug);
  // A few made in a row at most; nothing to sweep later.
  if (fresh.size > 20) fresh.delete(fresh.values().next().value as string);
}

export function isFresh(slug: string): boolean {
  return fresh.has(slug);
}
