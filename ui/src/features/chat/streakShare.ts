// V108-N23 The words of the streak prompt.

/** "5-stream watch streak. Share it?" Twitch counts streams in a row. */
export function streakText(value: number, fmt: (n: number) => string = String): string {
  return value > 0 ? `${fmt(value)}-stream watch streak. Share it?` : 'Share your watch streak?';
}
