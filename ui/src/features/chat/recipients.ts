// C16 A mass gift's recipients, one per line: the first few, the rest behind "+N more".

/** Names shown before the fold. */
export const RECIPIENTS_SHOWN = 10;

/** The names to draw and how many sit behind the fold. A fold of one name isn't worth a button. */
export function foldRecipients<T>(names: readonly T[], open: boolean, limit = RECIPIENTS_SHOWN): { shown: readonly T[]; hidden: number } {
  if (open || names.length <= limit + 1) return { shown: names, hidden: 0 };
  return { shown: names.slice(0, limit), hidden: names.length - limit };
}
