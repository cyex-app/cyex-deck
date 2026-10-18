// V1051-04 Pure helpers for the combos strip: which combo is worn, what's broken, how it reads.

import type { Combo } from '../../ipc/cosmetics';

export type Owned = 'usable' | 'needsSub' | 'notOwned';
export interface Problem {
  kind: 'paint' | 'badge';
  item: string;
  why: 'notOwned' | 'needsSub';
}

/** The first combo, in list order, that matches what's worn. */
export function current(combos: readonly Combo[], paint: string | null, badge: string | null): Combo | null {
  return combos.find((c) => (c.paint?.id ?? null) === paint && (c.badge?.id ?? null) === badge) ?? null;
}

export function problems(c: Combo, have: (kind: 'paint' | 'badge', id: string) => Owned): Problem[] {
  const out: Problem[] = [];
  for (const kind of ['paint', 'badge'] as const) {
    const part = c[kind];
    if (!part) continue;
    const s = have(kind, part.id);
    if (s !== 'usable') out.push({ kind, item: part.name, why: s });
  }
  return out;
}

export function problemText(p: readonly Problem[]): string {
  return p
    .map((x) => (x.why === 'notOwned' ? `You don’t own ${x.item} anymore.` : `${x.item} is for 7TV subscribers.`))
    .join(' ');
}

export function pairTitle(c: Combo): string {
  return `${c.paint?.name ?? 'No paint'} + ${c.badge?.name ?? 'No badge'}`;
}

/** The worn-already case has no note from Rust: add the broken-combo one. */
export const withNote = (line: string, p: readonly Problem[]): string =>
  p.length && !line.includes('without') ? `${line.replace(/\.$/, '')}. ${problemText(p)}` : line;
