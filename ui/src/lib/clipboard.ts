// Copying, with one kind of feedback everywhere: a green "<What> copied." or a red
// "Couldn’t copy that." Menus, buttons and players all go through here.
//
//   void copy(login, 'Name');        // "Name copied."

import { toast } from './toast.svelte.ts';

export type Write = (text: string) => Promise<void>;

const system: Write = (text) => navigator.clipboard.writeText(text);

/** Copies `text`; `what` names it for the toast ("Link", "Code"). True when it worked. */
export async function copy(text: string, what: string, write: Write = system): Promise<boolean> {
  try {
    await write(text);
    toast.success(`${what.charAt(0).toUpperCase()}${what.slice(1)} copied.`);
    return true;
  } catch {
    toast.error('Couldn’t copy that. Try again.');
    return false;
  }
}
