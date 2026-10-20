// V107-R29 Mod view panels in their own window, by route `modpanel/<login>/<panel>`. Each
// goes back where it came from when its window closes: popped out of the layout, into its
// exact place (the layout as it was then is kept in `modview.popped`); popped out of the
// closed-panels row, back there.

import { isSplit, type Node, prune, restore } from '../../lib/panels.ts';

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
