// V1051-02 The account dropdown's choices: alphabetical by display name, case-insensitive.

import type { Choice } from '../../components/select.ts';

interface Account {
  userId: string;
  displayName: string;
  avatar?: string | null;
}

export function accountChoices(list: readonly Account[]): Choice[] {
  return list
    .map((a) => ({ value: a.userId, label: a.displayName, picture: a.avatar ?? null }))
    .sort((a, b) => a.label.localeCompare(b.label, undefined, { sensitivity: 'base' }) || a.value.localeCompare(b.value));
}
