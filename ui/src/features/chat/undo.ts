// One-click actions that can be taken back: they run at once, and a toast offers Undo.

import { errorText } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';

type Step = () => Promise<unknown> | unknown;

/** Runs `run`, then says `text` with an Undo button that runs `undo`. A run that fails or
 *  answers `false` shows no toast of its own (the caller reports the error). */
export async function withUndo(text: string, run: Step, undo: Step): Promise<void> {
  try {
    if ((await run()) === false) return;
  } catch (e) {
    toast.error(errorText(e));
    return;
  }
  toast.info(text, {
    action: {
      label: 'Undo',
      run: () => {
        void Promise.resolve(undo()).catch((e) => toast.error(errorText(e)));
      },
    },
  });
}

/** A chat command runner: resolves `false` when it failed. */
export type Run = (line: string) => Promise<boolean> | void;

export const hideUser = (run: Run, login: string) =>
  withUndo(`${login} is hidden.`, () => run(`/hideuser ${login}`), () => run(`/unhideuser ${login}`));

export const blockUser = (run: Run, login: string) =>
  withUndo(`${login} is blocked.`, () => run(`/block ${login}`), () => run(`/unblock ${login}`));
