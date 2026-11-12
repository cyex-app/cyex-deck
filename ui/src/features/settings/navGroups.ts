// V110-N6 Settings sidebar groups that fold: which are folded is remembered per group on this PC.
// The group of the open page never folds away, and a search ignores folding.

const KEY = 'cyex.settingsGroups';

/** The folded groups' names. Anything unreadable counts as none folded. */
export function readFolded(): string[] {
  try {
    const v: unknown = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

export function saveFolded(folded: string[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(folded));
  } catch {
    // Not remembered; the fold still holds until restart.
  }
}

/** Is the group's page list shown? The open page's group, a search and a narrow list always show. */
export function groupShown(group: string, folded: readonly string[], current: string, forced: boolean): boolean {
  return forced || group === current || !folded.includes(group);
}

/** The group's name in or out of the folded list. */
export function toggleFolded(folded: readonly string[], group: string): string[] {
  return folded.includes(group) ? folded.filter((g) => g !== group) : [...folded, group];
}
