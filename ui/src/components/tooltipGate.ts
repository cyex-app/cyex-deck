// rb-1 Whether a tip may open: something to say, its `when` holds, and no drag is on
// (a grip's tip would reopen on every row the pointer passes and cover the ghost).

export interface TipGateOptions {
  label: string;
  image?: string;
  when?: () => boolean;
}

export function tipAllowed(opts: TipGateOptions, dragging: boolean): boolean {
  if (dragging) return false;
  if (!opts.label && !opts.image) return false;
  return !opts.when || opts.when();
}

/** V108-F59i Keys that close or confirm something, after which focus lands back on the trigger
 * by itself (Escape out of Settings, Enter in a menu): that focus is not the reader arriving there. */
const RESTORING_KEYS = new Set(['Escape', 'Enter', ' ', 'Delete']);

/** Whether focus that follows `lastKey` (null: a pointer came last) may open the tip. */
export function focusTipAllowed(lastKey: string | null): boolean {
  return lastKey === null || !RESTORING_KEYS.has(lastKey);
}
