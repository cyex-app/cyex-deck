// V106-31/32 Runs one 7TV edit: toast with Undo on success, the failure back to the caller.

import type { EditDone } from '../../../ipc/bindings/EditDone';
import type { EditFail } from '../../../ipc/bindings/EditFail';
import type { AddTarget } from '../../../ipc/bindings/AddTarget';
import type { EditOp } from '../../../ipc/bindings/EditOp';
import type { HereEdit } from '../../../ipc/bindings/HereEdit';
import { emotesIpc } from '../../../ipc/emotes';
import { toast } from '../../../lib/toast.svelte';
import { forgetEmoteMenu } from '../menus';
import { aliasOf, doneText, failText, inlineText, nameVerdict } from './words';

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

/** U4 What the user is filling in: add to a channel's set, or rename in this chat's. */
export type EditForm = { kind: 'add'; target: AddTarget; ownName?: string } | { kind: 'rename'; here: HereEdit };

export const formChannel = (f: EditForm): string => (f.kind === 'add' ? f.target.channel : f.here.channel);
const formSet = (f: EditForm): string => (f.kind === 'add' ? f.target.setId : f.here.setId);

/** U4 Whether `name` is free in the form's set. */
export async function checkName(account: string, form: EditForm, name: string): Promise<string | null> {
  const from = form.kind === 'rename' ? form.here.alias : null;
  return nameVerdict(await emotesIpc.editName(account, formSet(form), name, from), formChannel(form), form.kind === 'rename');
}

/** U4 Runs the form's add or rename. `onok` runs when it went through. Returns the inline line
 *  when the form stays open (null also for a failure that got a toast, with `onok` not called). */
export async function submitForm(
  account: string,
  login: string,
  emoteId: string,
  form: EditForm,
  name: string,
  onok?: () => void,
): Promise<string | null> {
  const channel = formChannel(form);
  const op: EditOp =
    form.kind === 'add'
      ? { type: 'add', setId: form.target.setId, emoteId, alias: name }
      : { type: 'rename', setId: form.here.setId, emoteId, from: form.here.alias, to: name };
  const fail = await runEdit(account, login, op, channel, { quietFail: (e) => inlineText(e, channel) !== null });
  if (!fail) onok?.();
  return fail ? inlineText(fail, channel) : null;
}

/** U4 Removes the emote from this chat's set. Returns the failure, or null. */
export function removeHere(account: string, login: string, emoteId: string, h: HereEdit): Promise<EditFail | null> {
  return runEdit(account, login, { type: 'remove', setId: h.setId, emoteId, alias: h.alias }, h.channel);
}
