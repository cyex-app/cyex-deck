// V106-33 (Q13) Duplicate rows after the Mentions merge: which they are, and dropping the later copies.

import type { MentionCheck } from '../../../ipc/bindings/MentionCheck';
import type { PhraseRule } from '../../../ipc/bindings/PhraseRule';

/** Rows that repeat an earlier one (Rust's sameAs), under the first. 0-based indexes, in list order. */
export function groups(list: PhraseRule[], checks: MentionCheck[]): { first: number; copies: number[] }[] {
  const byFirst = new Map<number, number[]>();
  for (let i = 0; i < list.length; i++) {
    const same = checks[i]?.sameAs;
    if (same === undefined) continue;
    const first = same - 1;
    if (first < 0 || first >= i) continue;
    const copies = byFirst.get(first);
    if (copies) copies.push(i);
    else byFirst.set(first, [i]);
  }
  return [...byFirst].sort((a, b) => a[0] - b[0]).map(([first, copies]) => ({ first, copies }));
}

/** O3: the first row of each group stays as it is; the later copies are deleted, their options with them. */
export function removeCopies(list: PhraseRule[], checks: MentionCheck[]): PhraseRule[] {
  const gone = new Set(groups(list, checks).flatMap((g) => g.copies));
  return list.filter((_, i) => !gone.has(i));
}
