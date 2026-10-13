// V105-12 After Add, the new row's first field takes the cursor.
import { tick } from 'svelte';

export async function focusLast(list: HTMLElement | undefined, row: string): Promise<void> {
  await tick();
  const rows = list?.querySelectorAll<HTMLElement>(row);
  const last = rows?.[rows.length - 1];
  last?.querySelector<HTMLElement>('input.field:not([type]), textarea.field, select.field')?.focus();
}
