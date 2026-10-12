// V103-13 A search result's setting: scrolled to the middle and lit up for a moment. The
// look is in SettingsView (`.found`, `.fading`).

const LIT_MS = 1400;
const FADE_MS = 700;

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
    }
    el.scrollIntoView({ block: 'center', behavior: 'smooth' });
    el.classList.add('found');
    const fade = window.setTimeout(() => el.classList.replace('found', 'fading'), LIT_MS);
    const done = window.setTimeout(() => el.classList.remove('fading'), LIT_MS + FADE_MS);
    last = { el, timers: [fade, done] };
    return true;
  }
  root.scrollTo({ top: 0 });
  return false;
}
