// V108-N21 Pure helpers for the "pick tabs to reopen" dialog after a crash.

/** The tab ids to keep: every tab not unticked, in list order. */
export function picked(all: string[], off: ReadonlySet<string>): string[] {
  return all.filter((id) => !off.has(id));
}

/** The button text for the current picks. */
export function reopenLabel(count: number): string {
  return count === 0 ? 'Start empty' : count === 1 ? 'Reopen 1 tab' : `Reopen ${count} tabs`;
}
