// V107-R12 After a crash, the next start says so once, with a way to the logs folder.

import { invoke } from '@tauri-apps/api/core';

import { errorText } from './errors';
import { toast } from './toast.svelte';

export async function showCrashNote(): Promise<void> {
  const crashed = await invoke<boolean>('crash_take_note').catch(() => false);
  if (!crashed) return;
  // Startup is busy: it stays long enough to be seen (and waits while hovered).
  toast.info('cyex deck closed unexpectedly. Details are in the logs folder.', {
    ms: 20_000,
    action: {
      label: 'Open folder',
      run: () => void invoke<void>('crash_open_folder').catch((e) => toast.error(errorText(e))),
    },
  });
}
