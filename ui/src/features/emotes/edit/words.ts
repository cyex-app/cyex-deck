// V106-31/32 What the 7TV edit flow says: toasts, inline form lines, greyed-out reasons.
// I106-3's Rust texts (chat/run/seventv_edit.rs) copy failText; keep the `${channel}` and `${alias}` names.

import type { AddBlocked } from '../../../ipc/bindings/AddBlocked';
import type { TargetBlocked as Blocked } from '../../../ipc/bindings/TargetBlocked';
import type { EditFail } from '../../../ipc/bindings/EditFail';
import type { EditOp } from '../../../ipc/bindings/EditOp';
import type { NameCheck } from '../../../ipc/bindings/NameCheck';

/** "30 s" under a minute, else whole minutes rounded up (same rule as cosmetics/words.rs). */
function wait(seconds: number): string {
  return seconds < 60 ? `${seconds} s` : `${Math.ceil(seconds / 60)} min`;
}

export function failText(e: EditFail, channel: string, alias: string): string {
  switch (e.type) {
    case 'signInAgain':
      return 'Your 7TV sign-in ran out. Sign in again in Settings → Accounts.';
    case 'notAllowed':
      return `You can’t edit ${channel}’s 7TV emotes.`;
    case 'full':
      return `${channel}’s 7TV emotes are full.`;
    case 'nameTaken':
      return `${channel} already has an emote named ${alias}.`;
    case 'private':
      return 'That emote is private. Only its owner can add it.';
    case 'notFound':
      return 'That emote isn’t there anymore.';
    case 'badName':
      return '7TV doesn’t allow that name.';
    case 'rateLimited':
      return `7TV needs a short break. Try again in ${wait(e.seconds)}.`;
    case 'down':
      return 'Couldn’t reach 7TV. Nothing changed.';
    case 'noConsent':
    case 'signedOut':
      return 'Sign in to 7TV first in Settings → Accounts.';
    case 'testRun':
      return '7TV edits only go to the test mock here.';
    default:
      return 'Something went wrong at 7TV. Nothing changed.';
  }
}

/** The name an op ends up with. */
export function aliasOf(op: EditOp): string {
  return op.type === 'rename' ? op.to : op.alias;
}

export function doneText(op: EditOp, channel: string): string {
  switch (op.type) {
    case 'add':
      return `Added ${op.alias} to ${channel}.`;
    case 'remove':
      return `Removed ${op.alias} from ${channel}.`;
    case 'rename':
      return `Renamed ${op.from} to ${op.to}.`;
  }
}

/** Why an item is greyed. With `channel`: one target's reason; without: the whole item's. */
export function blockedText(b: Blocked | AddBlocked, channel?: string): string {
  switch (b) {
    case 'already':
      return channel ? 'Already there' : 'Already in your channels';
    case 'private':
      return 'Private emote';
    case 'full':
      return 'Full';
    case 'nowhere':
      return 'No room in your channels';
  }
}

/** The form's own line; null means a toast is better. */
export function inlineText(e: EditFail, channel: string): string | null {
  switch (e.type) {
    case 'nameTaken':
      return `Taken in ${channel}.`;
    case 'badName':
      return '7TV doesn’t allow that name.';
    case 'full':
      return `${channel} is full.`;
    case 'private':
      return 'Private emote.';
    default:
      return null;
  }
}

/** U4 The form's line for a name check: null when free, else why not. A rename to its own name
 *  has nothing to save; the line shows once typed or sent (rc-4). Pure. */
export function nameVerdict(r: NameCheck, channel: string, rename: boolean): string | null {
  if (r === 'taken') return `Taken in ${channel}.`;
  return r === 'same' && rename ? 'That’s the current name.' : null;
}
