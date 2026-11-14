// V111-07 "Minimize join/part messages": a join or part line shows its first 5 names, with
// Show more / Show less beside them. Off, every name shows and there is no toggle.

/** Names shown while folded. */
export const PRESENCE_SHOWN = 5;

export interface PresenceFold<T> {
  /** The names to draw. */
  shown: readonly T[];
  /** How many sit behind Show more (0 when open or not folded). */
  hidden: number;
  /** The toggle's text; null when the line has nothing to fold. */
  toggle: 'Show more' | 'Show less' | null;
}

export function foldPresence<T>(names: readonly T[], minimize: boolean, open: boolean, limit = PRESENCE_SHOWN): PresenceFold<T> {
  if (!minimize || names.length <= limit) return { shown: names, hidden: 0, toggle: null };
  if (open) return { shown: names, hidden: 0, toggle: 'Show less' };
  return { shown: names.slice(0, limit), hidden: names.length - limit, toggle: 'Show more' };
}
