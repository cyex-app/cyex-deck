// V102-25 The usercard's flag for an account Twitch hides: suspended for a while or with
// no end, taken down for copyright, or closed by its owner. The card still opens with what
// Twitch, the chat and the logs know.

import type { Suspension } from '../../ipc/bindings/Suspension';

export interface SuspendedFlag {
  label: string;
  hint: string;
  tone: 'red' | 'yellow' | 'grey';
}

const FLAGS: Record<Suspension, SuspendedFlag> = {
  temporary: { label: 'Suspended · temporary', hint: 'Twitch suspended this account for a while.', tone: 'yellow' },
  indefinite: { label: 'Suspended · indefinite', hint: 'Twitch suspended this account with no end date.', tone: 'red' },
  dmca: { label: 'Suspended · DMCA', hint: 'Taken down over copyright claims.', tone: 'red' },
  deactivated: { label: 'Deactivated', hint: 'Closed by its owner.', tone: 'grey' },
  other: { label: 'Hidden on Twitch', hint: 'Twitch doesn’t show this account right now.', tone: 'grey' },
};

export function suspendedFlag(s: Suspension | null | undefined): SuspendedFlag | null {
  return s ? FLAGS[s] : null;
}
