// V110-N6 Settings sidebar groups that fold: which are folded is remembered per group on this PC.
// Every group folds, the open page's too (V110-N9); a search and the narrow list ignore folding,
// and going to a page some other way unfolds its group (`unfoldFor`).

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

/** Is the group's page list shown? A search and a narrow list always show; otherwise unless folded. */
export function groupShown(group: string, folded: readonly string[], forced: boolean): boolean {
  return forced || !folded.includes(group);
}

/** The folded list with `group` open: used when a page in it is reached by search or a jump. */
export function unfoldFor(folded: readonly string[], group: string): string[] {
  return folded.includes(group) ? folded.filter((g) => g !== group) : [...folded];
}

/** The group's name in or out of the folded list. */
export function toggleFolded(folded: readonly string[], group: string): string[] {
  return folded.includes(group) ? folded.filter((g) => g !== group) : [...folded, group];
}
