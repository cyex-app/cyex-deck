// V110-F83 Media errors may be recovered twice within a minute; older ones no longer count
// (the budget used to refill only on a new load, so the third error in a long session ended it).

export const RECOVER_WINDOW_MS = 60_000;
export const RECOVER_MAX = 2;

/** The recovery times still counting after this error, or null when the budget is spent. */
export function recoverMedia(times: number[], now: number): number[] | null {
  const recent = times.filter((t) => now - t < RECOVER_WINDOW_MS);
  if (recent.length >= RECOVER_MAX) return null;
  return [...recent, now];
}
