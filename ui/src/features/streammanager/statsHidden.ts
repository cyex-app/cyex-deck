// V108-C38 Which Stream card numbers are hidden by a click, as saved in
// `streammanager.hiddenStats`. Pure, so the toggle is testable.

export const STAT_IDS = ['viewers', 'uptime', 'followers', 'subs', 'subPoints'] as const;
export type StatId = (typeof STAT_IDS)[number];

/** The list after clicking `id`: hidden becomes shown and the other way round. */
export function toggleStat(hidden: readonly string[], id: StatId): string[] {
  const rest = hidden.filter((h) => h !== id);
  return rest.length === hidden.length ? [...hidden, id] : rest;
}

export const isHidden = (hidden: readonly string[], id: StatId) => hidden.includes(id);
