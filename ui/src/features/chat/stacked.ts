// V107-Mj Zalgo: a run of combining marks stacks far above or below its line and runs
// over its neighbours. Lines with such a run clip to their own box (Line.svelte). Real
// writing stacks two or three marks at most (Vietnamese, Thai, Devanagari), so those
// lines, and their emotes' modifiers, are left alone.

import type { Fragment } from '../../ipc/bindings/Fragment';

const RUN = /\p{M}{4,}/u;

/** A run of four or more combining marks in `text`. */
export const stacks = (text: string | null | undefined) => !!text && RUN.test(text);

/** Any text in the line stacks marks. */
export const lineStacks = (body: Fragment[], notice?: string | null) =>
  stacks(notice) || body.some((f) => 'text' in f && stacks(f.text));
