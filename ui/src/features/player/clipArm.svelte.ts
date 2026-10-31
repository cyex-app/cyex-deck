// V108-CC The player's Clip button asks twice: the first click arms it for 3 s ("Click again
// to clip"), the second makes the clip. Esc only disarms (a popover layer, as ConfirmButton).
// Call during component init: it registers its own effects.

import { layer } from '../../lib/escape';

export const ARM_MS = 3000;

export function useClipArm(run: () => void) {
  let armed = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function disarm() {
    clearTimeout(timer);
    armed = false;
  }

  $effect(() => () => clearTimeout(timer));
  $effect(() => {
    if (!armed) return;
    return layer('popover', disarm);
  });

  return {
    get armed() {
      return armed;
    },
    click() {
      if (!armed) {
        armed = true;
        timer = setTimeout(() => (armed = false), ARM_MS);
        return;
      }
      disarm();
      run();
    },
  };
}
