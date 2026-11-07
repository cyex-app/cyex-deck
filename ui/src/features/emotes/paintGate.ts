// V109-F78 A name wears a paint's class only once that paint's rules are in the sheet. The
// class alone (rules still waiting on pictures) draws the name in the plain currentcolor.
// Pure, so it's testable without runes.

/** The class for a paint, or `null` while its rules aren't in the sheet yet. */
export function paintClassOf(id: string | null, ready: ReadonlySet<string>): string | null {
  return id && ready.has(id) ? `paint-${id}` : null;
}
