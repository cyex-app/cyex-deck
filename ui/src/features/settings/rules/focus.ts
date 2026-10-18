// V105-12 After Add, the new row's first field takes the cursor.
import { tick } from 'svelte';

const FIELD = 'input.field:not([type]), textarea.field, select.field';

export async function focusLast(list: HTMLElement | undefined, row: string): Promise<void> {
  await tick();
  const rows = list?.querySelectorAll<HTMLElement>(row);
  const last = rows?.[rows.length - 1];
  last?.querySelector<HTMLElement>(FIELD)?.focus();
}

/** rb-6 After row `i` is deleted: the row now in its place, else the one above (its grip,
 *  else its first field), else `fallback` (e.g. the Add button). */
export async function focusAfterDelete(list: HTMLElement | undefined, row: string, i: number, fallback: string): Promise<void> {
  await tick();
  const rows = list?.querySelectorAll<HTMLElement>(row);
  const near = rows?.[Math.min(i, rows.length - 1)];
  const to = near?.querySelector<HTMLElement>('[data-drag-handle]') ?? near?.querySelector<HTMLElement>(FIELD);
  (to ?? list?.querySelector<HTMLElement>(fallback))?.focus();
}
