// V110-U71 The provider order from Settings (C11): lower is preferred. Rust uses the same list
// to pick which emote renders when two share a name.

import type { Provider } from '../../ipc/bindings/Provider';
import { settings } from '../../lib/settings.svelte';

/** A provider's place in `order`; ones missing from it come last. */
export function placeIn(order: readonly string[], p: Provider): number {
  const i = order.indexOf(p);
  return i < 0 ? order.length : i;
}

/** The place of each provider in the user's order. */
export function providerRank(): (p: Provider) => number {
  const order = settings.v.chat.emotePriority;
  return (p) => placeIn(order, p);
}
