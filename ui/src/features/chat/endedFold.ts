// V110-N6 An ended poll or prediction folds to its header 30 s after it ended and goes after 1 min.
// A live one never does. Unfolding it by hand keeps it open until the minute is up.

export const FOLD_AFTER_MS = 30_000;
export const CLOSE_AFTER_MS = 60_000;

export type EndPhase = 'open' | 'folded' | 'gone';

const POLL_ENDED = ['COMPLETED', 'TERMINATED', 'ARCHIVED'];
/** LOCKED is not an end: the result is still to come. */
const PREDICTION_ENDED = ['RESOLVED', 'CANCELED'];

export const pollEnded = (status: string): boolean => POLL_ENDED.includes(status);
export const predictionEnded = (status: string): boolean => PREDICTION_ENDED.includes(status);

/** Where an item stands `now`. `endedAt` is null while it is live. */
export function endPhase(endedAt: number | null, now: number, unfolded: boolean): EndPhase {
  if (endedAt === null) return 'open';
  const age = now - endedAt;
  if (age >= CLOSE_AFTER_MS) return 'gone';
  if (age >= FOLD_AFTER_MS && !unfolded) return 'folded';
  return 'open';
}
