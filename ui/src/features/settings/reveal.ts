// V103-13 A search result's setting: scrolled to the middle and lit up for a moment. The
// look is in SettingsView (`.found`, `.fading`). V110-U56 The row's control takes focus, and
// the tint holds briefly, then fades over about a second (not at all with Reduce motion).

import { reduced, scrollBehavior } from '../../lib/motion';
import { scrollWithin } from '../../lib/scrollWithin';
import { CONTROL_SELECTOR, litTimes } from './revealPlan';

let last: { el: HTMLElement; timers: number[] } | null = null;

/** Shows the first of `ids` marked in `root` (`data-setting`). False when none is on the
 * page; `root` then scrolls to the top. */
export function reveal(root: HTMLElement, ids: string[]): boolean {
  for (const id of ids) {
    const el = root.querySelector<HTMLElement>(`[data-setting~="${CSS.escape(id)}"]`);
    if (!el) continue;
    if (last) {
      last.timers.forEach(clearTimeout);
      last.el.classList.remove('found', 'fading');
      last.el.style.removeProperty('--found-fade');
    }
    const { holdMs, fadeMs } = litTimes(reduced());
    scrollWithin(el, 'center', scrollBehavior());
    el.style.setProperty('--found-fade', `${fadeMs}ms`);
    el.classList.add('found');
    focusControl(el);
    const fade = window.setTimeout(() => el.classList.replace('found', 'fading'), holdMs);
    const done = window.setTimeout(() => {
      el.classList.remove('fading');
      el.style.removeProperty('--found-fade');
    }, holdMs + fadeMs);
    last = { el, timers: [fade, done] };
    return true;
  }
  root.scrollTo({ top: 0 });
  return false;
}

/** The row itself when it is a control, else the first control inside it. A row with none
 * (a heading, a list) keeps focus where it was. */
function focusControl(el: HTMLElement) {
  const target = el.matches(CONTROL_SELECTOR) ? el : el.querySelector<HTMLElement>(CONTROL_SELECTOR);
  target?.focus({ preventScroll: true });
}
