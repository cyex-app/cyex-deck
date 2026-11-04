// V108-F60d Hide and the events strip swap places: focus goes to the control that replaces
// the pressed one, never to the page body. Both live in the chat's `.banners` block.
import { tick } from 'svelte';

const within = (from: Element | null, sel: string) =>
  from?.closest('.banners')?.querySelector<HTMLElement>(sel) ?? null;

/** After Hide: the "n Events" strip now stands where the banner was. */
export async function focusStrip(host: Element | null) {
  await tick();
  within(host, '.events')?.focus();
}

/** After the strip is pressed: the first banner it brought back (poll/prediction title, or the pin's close). */
export async function focusBanner(host: Element | null) {
  await tick();
  within(host, '.box .title, .pin .close')?.focus();
}
