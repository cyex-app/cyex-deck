// V1061-04/05 Which focused elements keep the keys, what a seek reports, the readout text.

import { keyFocused } from './focusOrigin.ts';
import { clock } from './time.ts';

/** Focused elements that use the arrow keys themselves. Buttons are left out on purpose:
 *  after a click on Play the seek must still work. A tabindex -1 slider (the SeekBar, focused
 *  only by a click) has no keys of its own, so it never keeps them. Tabs are below (d-2). */
export const OWN_ARROWS =
  'input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="slider"]:not([tabindex="-1"]), [role="listbox"], [role="option"], [role="grid"], [role="gridcell"], [role="menu"], [role="menubar"], [role^="menuitem"], [role="radiogroup"], [role="radio"], [role="tablist"], [role="tree"], [role="treegrid"], [role="spinbutton"], [role="combobox"], [role="log"], [data-arrows]';

/** Text fields keep every key. */
export const TEXT_FIELDS = 'input, textarea, select, [contenteditable]:not([contenteditable="false"])';

/** d-2 A tab moves focus along its own bar (TabItem: Left/Right, Up/Down when vertical), and
 *  only once the keyboard put focus there (rd-1 focusOrigin.ts): after a click the arrows seek. */
export const TAB_ACROSS = '[role="tab"]:not([data-vertical])';
export const TAB_DOWN = '[role="tab"][data-vertical]';

type Focus = { closest(sel: string): unknown } | null;

/** d-2 Whether the focused element keeps this key. J/L, Shift+arrows and the rest: only text
 *  fields keep them. Plain arrows: also the elements in OWN_ARROWS and a tab on its axis.
 *  `byKey`: whether the keyboard focused that tab. */
export function ownsKey(target: Focus, e: { key: string; shiftKey?: boolean }, byKey: (el: unknown) => boolean = keyFocused): boolean {
  if (!target?.closest) return false;
  if (target.closest(TEXT_FIELDS)) return true;
  if (!e.key.startsWith('Arrow') || e.shiftKey) return false;
  if (target.closest(OWN_ARROWS)) return true;
  const tab = target.closest(e.key === 'ArrowLeft' || e.key === 'ArrowRight' ? TAB_ACROSS : TAB_DOWN);
  return !!tab && byKey(tab);
}

/** A live seek's result: how far behind it lands ('live' = at live), 'start' when clamped at the stream's start. */
export type SeekResult = { behind: number | 'live'; edge: 'start' | null };

/** "-10s", "+30s", "-1:10" from 60 s on (VODs and clips). */
export function stepText(sec: number): string {
  const sign = sec < 0 ? '-' : '+';
  const a = Math.abs(Math.round(sec));
  return a >= 60 ? `${sign}${clock(a)}` : `${sign}${a}s`;
}

/** "-0:42", "-1:02:03". Live button and live readout. */
export function behindText(sec: number): string {
  return `-${clock(sec)}`;
}

/** Presses within 800 ms add up for the readout. */
export class Steps {
  private sum = 0;
  private at = Number.NEGATIVE_INFINITY;

  add(sec: number, now: number): number {
    this.sum = now - this.at <= 800 ? this.sum + sec : sec;
    this.at = now;
    return this.sum;
  }
}

/** Nearest allowed step: 5, 10, 15 or 30. */
export function stepChoice(n: number): 5 | 10 | 15 | 30 {
  const all = [5, 10, 15, 30] as const;
  const v = Number.isFinite(n) ? n : 10;
  return all.reduce((best, x) => (Math.abs(x - v) < Math.abs(best - v) ? x : best), 10 as 5 | 10 | 15 | 30);
}

/** The readout for a seek: a null result = not seekable (nothing shown). */
export function seekText(r: SeekResult | null | void, sum: number): string | null {
  if (r === null) return null;
  if (r === undefined) return stepText(sum);
  if (r.edge === 'start') return 'Can’t go back further';
  if (r.behind === 'live') return 'Live';
  return behindText(r.behind);
}

/** V1061-05 'skip' = a held key does nothing more; 'pass' = the key stays with the page. */
export function liveKeyAction(o: { repeat: boolean; owns: boolean; hasLive: boolean }): 'run' | 'skip' | 'pass' {
  if (o.owns || !o.hasLive) return 'pass';
  return o.repeat ? 'skip' : 'run';
}
