// V103-05, V103-06 One panel layout for tabs, the mod view and the stream manager: a tree of
// splits whose leaves are panel ids (`PanelNode`, stored by Rust). Plain functions, so they
// can be tested: moving, closing and bringing back panels, fitting a tree into a box and
// laying it out. `components/PanelLayout.svelte` draws it; every panel stays where it is in
// the page, so a playing stream never restarts when panels move.

import type { PanelNode } from '../ipc/bindings/PanelNode';
import type { SplitDir } from '../ipc/bindings/SplitDir';

export type Node = PanelNode;
export type Split = Extract<PanelNode, { dir: SplitDir }>;
export type Edge = 'left' | 'right' | 'top' | 'bottom';

export interface Box {
  w: number;
  h: number;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** How far past a limit the space has to grow before a turned split turns back (V102-24). */
export const MARGIN = 48;

export const isSplit = (n: Node): n is Split => 'dir' in n;
export const leaf = (panel: string): Node => ({ panel });
export const axisOf = (e: Edge): SplitDir => (e === 'left' || e === 'right' ? 'row' : 'column');
const first = (e: Edge) => e === 'left' || e === 'top';

export function split(dir: SplitDir, parts: Node[], sizes?: number[]): Node {
  return { dir, parts, sizes: sizes ?? parts.map(() => 1 / parts.length) };
}

/** Panel ids in page order. */
export function leaves(n: Node | null): string[] {
  if (!n) return [];
  return isSplit(n) ? n.parts.flatMap(leaves) : [n.panel];
}

export const has = (n: Node | null, id: string) => leaves(n).includes(id);

/** Shares that add up to one; bad ones become even. */
function shares(sizes: number[], count: number): number[] {
  const s = Array.from({ length: count }, (_, i) => (Number.isFinite(sizes[i]) && sizes[i] > 0 ? sizes[i] : 1));
  const total = s.reduce((a, b) => a + b, 0);
  return s.map((x) => x / total);
}

/** Empty and one-part splits fold away; a split inside one going the same way joins it;
 * repeated panels go. Same rules as Rust's `panels::sanitize`. */
export function normalize(n: Node | null, seen = new Set<string>()): Node | null {
  if (!n) return null;
  if (!isSplit(n)) {
    if (!n.panel || seen.has(n.panel)) return null;
    seen.add(n.panel);
    return n;
  }
  const sizes = shares(n.sizes, n.parts.length);
  const parts: Node[] = [];
  const out: number[] = [];
  n.parts.forEach((p, i) => {
    const c = normalize(p, seen);
    if (!c) return;
    if (isSplit(c) && c.dir === n.dir) {
      c.parts.forEach((q, j) => {
        parts.push(q);
        out.push(c.sizes[j] * sizes[i]);
      });
    } else {
      parts.push(c);
      out.push(sizes[i]);
    }
  });
  if (!parts.length) return null;
  if (parts.length === 1) return parts[0];
  return { dir: n.dir, parts, sizes: shares(out, parts.length) };
}

/** Without the panel `id`. */
export function remove(n: Node | null, id: string): Node | null {
  return prune(n, (p) => p !== id);
}

/** Only the panels `keep` says yes to. */
export function prune(n: Node | null, keep: (id: string) => boolean): Node | null {
  const cut = (x: Node): Node | null => {
    if (!isSplit(x)) return keep(x.panel) ? x : null;
    const kept = x.parts.map(cut);
    return { dir: x.dir, parts: kept.filter((p): p is Node => p !== null), sizes: x.sizes.filter((_, i) => kept[i] !== null) };
  };
  return n ? normalize(cut(n)) : null;
}

/** `id` on the `edge` side of the panel `target`, taking `share` of its room. With no
 * target (or one that isn't there) it goes along that edge of the whole layout. */
export function insert(n: Node | null, target: string | null, edge: Edge, id: string, share = 0.5): Node {
  const tree = remove(n, id);
  const add = leaf(id);
  const s = Math.min(Math.max(share, 0.1), 0.9);
  const pair = (other: Node): Node =>
    split(axisOf(edge), first(edge) ? [add, other] : [other, add], first(edge) ? [s, 1 - s] : [1 - s, s]);
  if (!tree) return add;
  if (target === null || !has(tree, target)) return normalize(pair(tree))!;
  const walk = (x: Node): Node => {
    if (!isSplit(x)) return x.panel === target ? pair(x) : x;
    const i = x.parts.findIndex((p) => !isSplit(p) && p.panel === target);
    if (i >= 0 && x.dir === axisOf(edge)) {
      // A sibling in the same split, taking its share from the target.
      const parts = [...x.parts];
      const sizes = [...x.sizes];
      const at = first(edge) ? i : i + 1;
      const given = sizes[i] * s;
      sizes[i] -= given;
      parts.splice(at, 0, add);
      sizes.splice(at, 0, given);
      return { dir: x.dir, parts, sizes };
    }
    return { dir: x.dir, parts: x.parts.map(walk), sizes: x.sizes };
  };
  return normalize(walk(tree))!;
}

/** How much of the whole layout the panel `id` has (its shares multiplied out). */
export function fraction(n: Node | null, id: string): number {
  if (!n) return 0;
  if (!isSplit(n)) return n.panel === id ? 1 : 0;
  return n.parts.reduce((a, p, i) => a + n.sizes[i] * fraction(p, id), 0);
}

/** The panel `id` dropped on the `edge` of `target` (V102-23). It takes the part of the
 * target's room it had of theirs together, so a 28 % chat stays 28 % next to its stream. */
export function move(n: Node | null, id: string, target: string | null, edge: Edge): Node | null {
  if (target === id) return n;
  const own = fraction(n, id);
  const other = target ? fraction(n, target) : 1 - own;
  const share = own > 0 && other > 0 ? own / (own + other) : 0.5;
  return insert(n, target, edge, id, Math.min(Math.max(share, 0.15), 0.85));
}

/** Where `id` sits in `full` (the default with every panel), as a panel to put it next to
 * in `n` and on which side: its nearest neighbour in the default that `n` still has. */
function slotOf(n: Node | null, id: string, full: Node): { target: string | null; edge: Edge; share: number } {
  const path = pathTo(full, id);
  // Inside out: the split right around it first.
  for (let depth = path.length - 1; depth >= 0; depth--) {
    const s = nodeAt(full, path.slice(0, depth)) as Split;
    const i = path[depth];
    for (let d = 1; d < s.parts.length; d++) {
      for (const j of [i - d, i + d]) {
        if (j < 0 || j >= s.parts.length) continue;
        const near = leaves(s.parts[j]).filter((l) => has(n, l));
        if (!near.length) continue;
        const before = j < i;
        const edge: Edge = s.dir === 'row' ? (before ? 'right' : 'left') : before ? 'bottom' : 'top';
        const share = s.sizes[i] / (s.sizes[i] + s.sizes[j]);
        return { target: before ? near[near.length - 1] : near[0], edge, share };
      }
    }
  }
  return { target: null, edge: 'right', share: 0.5 };
}

/** V103-06 A closed panel back in its default slot: next to the panel it sits beside in
 * `full`. */
export function restore(n: Node | null, id: string, full: Node): Node {
  if (n && has(n, id)) return n;
  const slot = slotOf(n, id, full);
  return insert(n, slot.target, slot.edge, id, slot.share);
}

/** V103-22 `id` as a whole column (or row) along `edge` of the layout with at least `share`
 * of it: left where it is when it's one already, else moved there. */
export function extend(n: Node | null, id: string, edge: Edge, share: number): Node {
  const s = Math.min(Math.max(share, 0.1), 0.9);
  if (n && !isSplit(n) && n.panel === id) return n;
  if (n && isSplit(n) && n.dir === axisOf(edge)) {
    const i = n.parts.findIndex((p) => !isSplit(p) && p.panel === id);
    if (i >= 0) {
      if (n.sizes[i] >= s) return n;
      // Grown to its share; the others give it up in proportion.
      const rest = 1 - n.sizes[i];
      const sizes = n.sizes.map((x, j) => (j === i ? s : rest > 0 ? (x * (1 - s)) / rest : (1 - s) / (n.parts.length - 1)));
      return { ...n, sizes };
    }
  }
  return insert(n, null, edge, id, s);
}

/** V103-22 Whether a view's own chat panel shows: as the user kept it (`kept`, brought back
 * or closed by hand), else only while the channel's regular chat shows nowhere else.
 * `regular` is `null` until that's known; the chat stays out meanwhile, so it never joins
 * and leaves again. */
export function chatShows(kept: boolean | null, regular: boolean | null): boolean {
  return kept ?? regular === false;
}

/** The layout with the chat panel `id` shown or not: where the layout has it, else as a
 * whole column along `edge` (`extend`). */
export function withChat(n: Node | null, id: string, show: boolean, edge: Edge, share: number): Node | null {
  if (!show) return remove(n, id);
  return has(n, id) ? n : extend(n, id, edge, share);
}

/** Panels the layout doesn't have yet, each put back where `full` has it. */
export function ensure(n: Node | null, ids: string[], full: Node): Node | null {
  return ids.reduce<Node | null>((t, id) => restore(t, id, full), n);
}

function pathTo(n: Node, id: string): number[] {
  if (!isSplit(n)) return [];
  for (let i = 0; i < n.parts.length; i++) {
    if (has(n.parts[i], id)) return [i, ...pathTo(n.parts[i], id)];
  }
  return [];
}

export function nodeAt(n: Node, path: number[]): Node | null {
  let x: Node | null = n;
  for (const i of path) x = x && isSplit(x) ? (x.parts[i] ?? null) : null;
  return x;
}

/** The same tree with new sizes for the split at `path`. */
export function setSizes(n: Node, path: number[], sizes: number[]): Node {
  if (!isSplit(n)) return n;
  if (!path.length) return { ...n, sizes: shares(sizes, n.parts.length) };
  const [i, ...rest] = path;
  return { ...n, parts: n.parts.map((p, j) => (j === i ? setSizes(p, rest, sizes) : p)) };
}

/** The same tree with new sizes for the split whose `splitKey` is `key`, if it has one. */
export function setSizesByKey(n: Node, key: string, sizes: number[]): Node {
  if (!isSplit(n)) return n;
  if (splitKey(n) === key && sizes.length === n.parts.length) return { ...n, sizes: shares(sizes, n.parts.length) };
  return { ...n, parts: n.parts.map((p) => setSizesByKey(p, key, sizes)) };
}

const setKey = (ids: string[]) => [...ids].sort().join('\u0001');

/** A split's identity, the same in the stored tree and the fitted one: its panels. */
export function splitKey(s: Split): string {
  return s.parts.map((p) => setKey(leaves(p))).join('\u0002');
}

/** The split in `user` that the split at `path` in `shown` (fitted from it) stands for:
 * the same panels in each part, the same way round. `null` when fitting turned it or
 * panels are missing, so a drag on it can't be kept. */
export function matchSplit(user: Node, shown: Node, path: number[]): number[] | null {
  const s = nodeAt(shown, path);
  if (!s || !isSplit(s)) return null;
  const want = s.parts.map((p) => setKey(leaves(p)));
  const visible = new Set(leaves(shown));
  const find = (x: Node, at: number[]): number[] | null => {
    if (!isSplit(x)) return null;
    const got = x.parts.map((p) => setKey(leaves(p).filter((l) => visible.has(l))));
    if (x.dir === s.dir && got.length === want.length && got.every((g, i) => g === want[i])) return at;
    for (let i = 0; i < x.parts.length; i++) {
      const r = find(x.parts[i], [...at, i]);
      if (r) return r;
    }
    return null;
  };
  return find(user, []);
}

/** The least room a tree needs: side by side the widths add up, else the widest. */
export function minOf(n: Node, min: (id: string) => Box): Box {
  if (!isSplit(n)) return min(n.panel);
  const m = n.parts.map((p) => minOf(p, min));
  const row = n.dir === 'row';
  return {
    w: row ? m.reduce((a, b) => a + b.w, 0) : Math.max(...m.map((b) => b.w)),
    h: row ? Math.max(...m.map((b) => b.h)) : m.reduce((a, b) => a + b.h, 0),
  };
}

/** Shares along `len` px with each part at least its `mins`; parts that need more take it
 * from the others in proportion. Too little room for all: each gets its part of it. */
export function clampSizes(sizes: number[], mins: number[], len: number): number[] {
  const want = shares(sizes, mins.length);
  const need = mins.reduce((a, b) => a + b, 0);
  if (len <= 0) return want;
  if (need >= len) return need > 0 ? mins.map((m) => m / need) : want;
  const fixed = new Set<number>();
  let out = want.map((s) => s * len);
  for (let round = 0; round < mins.length; round++) {
    const short = out.findIndex((px, i) => !fixed.has(i) && px < mins[i] - 0.01);
    if (short < 0) break;
    fixed.add(short);
    const taken = [...fixed].reduce((a, i) => a + mins[i], 0);
    const free = want.reduce((a, s, i) => (fixed.has(i) ? a : a + s), 0);
    out = want.map((s, i) => (fixed.has(i) ? mins[i] : free > 0 ? ((len - taken) * s) / free : 0));
  }
  return out.map((px) => px / len);
}

export interface FitOptions {
  min: (id: string) => Box;
  /** A two-part split too small its own way may turn round (V102-24). */
  turn?: boolean;
  /** Splits turned last time (`splitKey`): they turn back only with `MARGIN` to spare. */
  turned?: Set<string>;
  /** The shares of a split that turned, e.g. a stream at its 16:9 height above a chat. */
  turnedSizes?: (s: Split, box: Box) => number[] | null;
  /** As given to `layout`. */
  gap?: number;
  pad?: number;
}

export interface Fitted {
  tree: Node;
  /** The room it's laid out in: more than the box when it doesn't fit (it scrolls). */
  box: Box;
  /** The whole tree fits the box (turned splits included). */
  fits: boolean;
  turned: Set<string>;
}

/** `n` in `box`: every split's shares clamped so each panel gets its least room where there
 * is room, two-part splits turned round where that's what fits. Nothing here is saved. */
export function fitTree(n: Node, outer: Box, o: FitOptions): Fitted {
  // Worked out in `layout`'s own box: a gap larger, each panel a gap larger.
  const gap = o.gap ?? 0;
  const pad = o.pad ?? 0;
  const box = { w: outer.w - 2 * pad + gap, h: outer.h - 2 * pad + gap };
  const least = (id: string) => {
    const m = o.min(id);
    return { w: m.w + gap, h: m.h + gap };
  };
  const turned = new Set<string>();
  const place = (x: Node, b: Box, turn: boolean): Node => {
    if (!isSplit(x)) return x;
    let dir = x.dir;
    let sizes = x.sizes;
    const key = splitKey(x);
    const mins = x.parts.map((p) => minOf(p, least));
    const fitsWay = (d: SplitDir, margin: number) => {
      const row = d === 'row';
      const along = mins.reduce((a, m) => a + (row ? m.w : m.h), 0);
      const across = Math.max(...mins.map((m) => (row ? m.h : m.w)));
      return along + margin <= (row ? b.w : b.h) && across <= (row ? b.h : b.w);
    };
    const was = o.turned?.has(key) ?? false;
    if (turn && x.parts.length === 2 && !fitsWay(x.dir, was ? MARGIN : 0)) {
      const other: SplitDir = x.dir === 'row' ? 'column' : 'row';
      if (fitsWay(other, was ? -MARGIN : 0)) {
        dir = other;
        turned.add(key);
        sizes = o.turnedSizes?.(x, b) ?? sizes;
      }
    }
    const row = dir === 'row';
    const len = row ? b.w : b.h;
    sizes = clampSizes(sizes, mins.map((m) => (row ? m.w : m.h)), len);
    const parts = x.parts.map((p, i) =>
      place(p, row ? { w: b.w * sizes[i], h: b.h } : { w: b.w, h: b.h * sizes[i] }, turn),
    );
    return { dir, parts, sizes };
  };
  const tree = place(n, box, o.turn ?? false);
  const need = minOf(tree, least);
  const fits = need.w <= box.w + 0.5 && need.h <= box.h + 0.5;
  if (fits) return { tree, box: outer, fits, turned };
  // Too small even so: laid out at its least size, scrolling, turned as it is.
  const big = { w: Math.max(box.w, need.w), h: Math.max(box.h, need.h) };
  return { tree: place(tree, big, false), box: { w: big.w + 2 * pad - gap, h: big.h + 2 * pad - gap }, fits, turned };
}

/** Too narrow for the layout: every panel one under the other at `height(id)`, scrolling
 * (like the mod view's old one-column grid). */
export function stacked(n: Node, width: number, height: (id: string) => number, gap = 0, pad = 0): Fitted {
  const ids = leaves(n);
  const hs = ids.map((id) => height(id) + gap);
  const total = hs.reduce((a, b) => a + b, 0) || 1;
  return {
    tree: ids.length === 1 ? leaf(ids[0]) : { dir: 'column', parts: ids.map(leaf), sizes: hs.map((h) => h / total) },
    box: { w: width, h: total - gap + 2 * pad },
    fits: false,
    turned: new Set(),
  };
}

/** A divider between two parts of a split. */
export interface Divider {
  /** The split's path in the tree. */
  path: number[];
  /** Between part `index - 1` and part `index`. */
  index: number;
  dir: SplitDir;
  /** Where the line is, across the split. */
  at: number;
  /** The split's own box. */
  rect: Rect;
}

export interface Laid {
  panels: Map<string, Rect>;
  dividers: Divider[];
}

/** Every panel's box in `box`, with `gap` px between panels and `pad` px around them. */
export function layout(n: Node | null, box: Box, gap = 0, pad = 0): Laid {
  const panels = new Map<string, Rect>();
  const dividers: Divider[] = [];
  if (!n) return { panels, dividers };
  // Laid out in a box `gap` larger, then each panel gives back half a gap on every side.
  const half = gap / 2;
  const walk = (x: Node, r: Rect, path: number[]) => {
    if (!isSplit(x)) {
      panels.set(x.panel, { x: r.x + half, y: r.y + half, w: Math.max(0, r.w - gap), h: Math.max(0, r.h - gap) });
      return;
    }
    const row = x.dir === 'row';
    let pos = row ? r.x : r.y;
    x.parts.forEach((p, i) => {
      const len = (row ? r.w : r.h) * x.sizes[i];
      if (i > 0) dividers.push({ path, index: i, dir: x.dir, at: pos, rect: r });
      walk(p, row ? { x: pos, y: r.y, w: len, h: r.h } : { x: r.x, y: pos, w: r.w, h: len }, [...path, i]);
      pos += len;
    });
  };
  walk(n, { x: pad - half, y: pad - half, w: box.w - 2 * pad + gap, h: box.h - 2 * pad + gap }, []);
  return { panels, dividers };
}

/** New shares for a split whose divider `index` was dragged to `at` px (in the split's own
 * axis), keeping both parts beside it at least `mins` (px, per part). */
export function dragSizes(s: Split, rect: Rect, index: number, at: number, mins: number[]): number[] {
  const row = s.dir === 'row';
  const len = row ? rect.w : rect.h;
  if (len <= 0) return s.sizes;
  const start = row ? rect.x : rect.y;
  const before = s.sizes.slice(0, index - 1).reduce((a, b) => a + b, 0);
  const pair = s.sizes[index - 1] + s.sizes[index];
  const lo = before + Math.min(mins[index - 1] / len, pair / 2);
  const hi = before + pair - Math.min(mins[index] / len, pair / 2);
  const cut = Math.min(Math.max((at - start) / len, lo), hi);
  const sizes = [...s.sizes];
  sizes[index - 1] = cut - before;
  sizes[index] = pair - sizes[index - 1];
  return sizes;
}

/** A layout patch for the settings store, which merges objects (RFC 7386): the keys of the
 * other kind of node are cleared, so a panel never keeps a split's fields or back. */
export function asPatch(n: Node | null): Record<string, unknown> | null {
  if (!n) return null;
  return isSplit(n) ? { ...n, panel: null } : { ...n, dir: null, parts: null, sizes: null };
}
