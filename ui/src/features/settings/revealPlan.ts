// V110-U56 How a found setting lights up and where the keyboard goes. Pure, so it is testable.

/** How long the tint holds, then how long it fades (ms). Reduced motion: it holds, then goes at once. */
export function litTimes(reducedMotion: boolean): { holdMs: number; fadeMs: number } {
  return reducedMotion ? { holdMs: 1200, fadeMs: 0 } : { holdMs: 500, fadeMs: 1000 };
}

/** The controls a found row hands focus to, first match in the row wins. */
export const CONTROL_SELECTOR = [
  'input:not([type="hidden"]):not(:disabled)',
  'select:not(:disabled)',
  'textarea:not(:disabled)',
  'button:not(:disabled)',
  '[role="switch"]:not([aria-disabled="true"])',
  '[role="slider"]',
  'a[href]',
].join(',');
