// V110-A-idea Copy with a check mark on the button instead of a toast. A failure still toasts.
//   const cp = copied();   <IconButton icon={cp.is('name') ? 'check' : 'copy'} onclick={() => cp.run(text, 'Name', 'name')} />
//   <span class="sr-only" aria-live="polite">{cp.said}</span>
import { onDestroy } from 'svelte';

import { copy } from './clipboard.ts';
import { copiedSaid, copyFlash } from './copyFlash.ts';

export function copied() {
  let key = $state<string | null>(null);
  let said = $state('');
  const flash = copyFlash((k) => {
    key = k;
    if (k === null) said = '';
  });
  onDestroy(flash.stop);
  return {
    is: (k: string) => key === k,
    get said() {
      return said;
    },
    async run(text: string, what: string, k: string): Promise<void> {
      if (!(await copy(text, what, undefined, true))) return;
      said = copiedSaid(what);
      flash.show(k);
    },
  };
}
