// V108-N23 The words of the streak row.

/** "5-stream watch streak". Twitch counts streams in a row; the Share button asks the rest. */
export function streakText(value: number, fmt: (n: number) => string = String): string {
  return value > 0 ? `${fmt(value)}-stream watch streak` : 'Watch streak';
}

/** The narrow chat's version: "5-stream streak". */
export function streakShort(value: number, fmt: (n: number) => string = String): string {
  return value > 0 ? `${fmt(value)}-stream streak` : 'Streak';
}
