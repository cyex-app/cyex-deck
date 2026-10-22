// V107-R29 Mod view panels in their own window, by route `modpanel/<login>/<panel>`. Each
// goes back where it came from when its window closes: popped out of the layout, into its
// exact place (the layout as it was then is kept in `modview.popped`); popped out of the
// closed-panels row, back there.

import { has, isSplit, leaves, type Node, prune, restore } from '../../lib/panels.ts';

export const KIND = 'modpanel';

export const panelRoute = (login: string, id: string) => `${KIND}/${login}/${id}`;

/** The `modview.popped` key of a channel's panel. */
export const poppedKey = (login: string, id: string) => `${login}/${id}`;

export function parseRoute(route: string): { login: string; id: string } | null {
  const [kind, login, id] = route.split('/');
  return kind === KIND && login && id ? { login, id } : null;
}

/** The channel's panels open in a window of their own. */
export function poppedFor(routes: string[], login: string): Set<string> {
  const out = new Set<string>();
  for (const r of routes) {
    const p = parseRoute(r);
    if (p && p.login === login) out.add(p.id);
  }
  return out;
}

/** What the mod view shows: the layout without its popped-out panels. */
export function shownTree(tree: Node | null, out: Set<string>): Node | null {
  return out.size ? prune(tree, (id) => !out.has(id)) : tree;
}

/** The shape and shares of a layout, so a saved copy (other key order, rounding) compares. */
function canon(n: Node | null): string {
  if (!n) return '';
  if (!isSplit(n)) return n.panel;
  const sum = n.sizes.reduce((a, b) => a + b, 0) || 1;
  return `${n.dir}(${n.parts.map((p, i) => `${canon(p)}:${Math.round((n.sizes[i] / sum) * 1000)}`).join(',')})`;
}

const same = (a: Node | null, b: Node | null) => canon(a) === canon(b);

/** `id` back into `tree` from `was`, the layout when it popped out. Nothing changed since
 *  (panels still out aside): exactly that layout. Else next to the panel it sat beside,
 *  with its share of that room (`exact` false). */
export function dockBack(tree: Node | null, id: string, was: Node, stillOut: Set<string>): { tree: Node; exact: boolean } {
  const then = prune(was, (p) => !stillOut.has(p) || p === id);
  if (then && same(prune(then, (p) => p !== id), tree)) return { tree: then, exact: true };
  return { tree: restore(tree, id, was), exact: false };
}

/** The keys of `popped` whose window is closed, once the open windows are known. */
export function closedOut(popped: Record<string, unknown>, routes: string[]): string[] {
  const open = new Set(routes);
  return Object.keys(popped).filter((k) => !open.has(`${KIND}/${k}`));
}

/** V107-N3e The layout as it would be with every panel still out back in (`out`: id → its
 *  layout when it popped out, oldest first). Kept for the next panel that pops out, so a
 *  later one docks back into the whole layout: two panels out and back, in any order, end
 *  up exactly where they were (the second one's sizes drifted before). */
export function withPopped(tree: Node | null, out: [id: string, was: Node][]): Node | null {
  let t = tree;
  const left = new Set(out.map(([id]) => id));
  for (const [id, was] of out) {
    left.delete(id);
    if (t && !leaves(t).includes(id)) t = dockBack(t, id, was, left).tree;
  }
  return t;
}

const idOf = (key: string) => key.slice(key.indexOf('/') + 1);

/** V107-Mg This channel's panels in a window now (`out`) that are in the layout `tree`. */
export function mineOut(tree: Node | null, out: Set<string>): string[] {
  return leaves(tree).filter((id) => out.has(id));
}

/** V107-Mg Pop-outs are per channel; the layout is shared and keeps every panel, so other
 *  channels' views show it in its spot. What this channel saves (`next`, made from its view
 *  without its pop-outs `ids`) gets them back in where they sit in `tree`, the layout before. */
export function keepOut(next: Node | null, ids: string[], tree: Node | null): Node | null {
  if (!ids.length || !tree) return next;
  // Every panel left here closed: just the ones out.
  if (!next) return prune(tree, (id) => ids.includes(id));
  return withPopped(next, ids.map((id) => [id, tree]));
}

/** V107-Mg The entries to drop (window closed) and the layout with every entry's panel in
 *  it: one closed elsewhere comes back when its window closes; one missing from the layout
 *  while its window is open (saved before V107-Mg, when popping out left the shared layout)
 *  is put back so other channels show it. `null` tree: nothing changes. `wait`: keys whose
 *  window is about to open. */
export function settle(
  tree: Node | null,
  popped: Record<string, Node>,
  routes: string[],
  wait: Set<string>,
): { tree: Node | null; drop: string[] } {
  const drop = closedOut(popped, routes).filter((k) => !wait.has(k));
  const missing = Object.entries(popped)
    .filter(([k, was]) => was && !has(tree, idOf(k)))
    .map(([k, was]) => [idOf(k), was] as [string, Node]);
  const ids = new Set<string>();
  const once = missing.filter(([id]) => !ids.has(id) && ids.add(id));
  return { tree: once.length ? withPopped(tree, once) : null, drop };
}

/** V107-Mg A panel closed in one channel's view is closed for all: its pop-outs elsewhere
 *  go back to the closed row when their windows close. */
export const entriesOf = (popped: Record<string, unknown>, id: string) => Object.keys(popped).filter((k) => idOf(k) === id);
