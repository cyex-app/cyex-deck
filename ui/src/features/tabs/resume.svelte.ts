// V108-N21 After a crash the tabs are already back; this asks which ones to keep.

import { tabsIpc } from '../../ipc/tabs';
import { errorText } from '../../lib/errors';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';

export const resumeDialog = $state({ open: false });

/** Once per start, in the main window: opens the dialog if the last run didn't end cleanly. */
export async function askResume(): Promise<void> {
  const pending = await tabsIpc.resumePending().catch(() => false);
  if (pending) resumeDialog.open = true;
}

/** Keeps `keep`; the rest wait in "Reopen closed tab". */
export async function resumeWith(keep: string[]): Promise<void> {
  const n = settings.begin('tabs');
  try {
    const applied = await tabsIpc.resume(keep);
    await settings.finish('tabs', n, applied.tabs);
  } catch (e) {
    await settings.finish('tabs', n, undefined, false);
    toast.error(errorText(e));
  }
}
