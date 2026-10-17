// V106-31/32 Runs one 7TV edit: toast with Undo on success, the failure back to the caller.

import type { EditDone } from '../../../ipc/bindings/EditDone';
import type { EditFail } from '../../../ipc/bindings/EditFail';
import type { EditOp } from '../../../ipc/bindings/EditOp';
import { emotesIpc } from '../../../ipc/emotes';
import { toast } from '../../../lib/toast.svelte';
import { forgetEmoteMenu } from '../menus';
import { aliasOf, doneText, failText } from './words';

/** The IPC rejects with the EditFail object; anything else counts as `bad`. */
function asFail(e: unknown): EditFail {
  return e && typeof e === 'object' && 'type' in e ? (e as EditFail) : { type: 'bad' };
}

/** Runs one edit; on success a toast with Undo. Returns the failure for the caller to show inline, or null. */
export async function runEdit(
  account: string,
  login: string,
  op: EditOp,
  channel: string,
  opts?: { quietFail?: (e: EditFail) => boolean },
): Promise<EditFail | null> {
  let done: EditDone;
  try {
    done = await emotesIpc.edit(account, op);
  } catch (raw) {
    const err = asFail(raw);
    // signInAgain: Rust already told the accounts view.
    if (!opts?.quietFail?.(err)) toast.error(failText(err, channel, aliasOf(op)));
    return err;
  }
  editDone(account, login, op, done);
  return null;
}

async function undoEdit(account: string, login: string, done: EditDone): Promise<void> {
  try {
    await emotesIpc.edit(account, done.undo);
    forgetEmoteMenu(login);
  } catch (raw) {
    toast.error(failText(asFail(raw), done.channel, aliasOf(done.undo)));
  }
}

/** I106-3 The success half of runEdit, also used by ChatView for /add, /remove, /rename. */
export function editDone(account: string, login: string, op: EditOp, done: EditDone): void {
  forgetEmoteMenu(login);
  toast.success(doneText(op, done.channel), {
    action: { label: 'Undo', run: () => void undoEdit(account, login, done) },
  });
}
