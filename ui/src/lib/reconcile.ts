// V108-P1 A settings value from Rust lands in place: only leaves that differ are written, so
// unchanged objects keep their identity and only the changed signals of a $state proxy fire.
// A tab switch used to swap every Tab and Tile, a chat setting every chat line's settings.

import { sameJson } from './mergePatch.ts';

type Obj = Record<string, unknown>;

const isObj = (v: unknown): v is Obj => v !== null && typeof v === 'object' && !Array.isArray(v);

/** Writes `next` into `parent[key]`, changing only what differs. `next` must be plain JSON
 * that shares no objects with `parent` (snapshot it first if it may hold its proxies). */
export function reconcile(parent: Obj | unknown[], key: string | number, next: unknown): void {
  const p = parent as Obj;
  const cur = p[key];
  if (cur === next) return;
  if (isObj(cur) && isObj(next)) {
    reconcileObject(cur, next);
  } else if (Array.isArray(cur) && Array.isArray(next)) {
    const out = reconcileArray(cur, next);
    if (out) p[key] = out;
  } else {
    p[key] = next;
  }
}

function reconcileObject(cur: Obj, next: Obj): void {
  for (const k of Object.keys(cur)) if (!Object.hasOwn(next, k)) delete cur[k];
  for (const k of Object.keys(next)) reconcile(cur, k, next[k]);
}

/** A new array to assign, or null when `cur` was brought up to date in place. */
function reconcileArray(cur: unknown[], next: unknown[]): unknown[] | null {
  const had = byId(cur);
  if (had && byId(next)) {
    // Rows with ids (tabs, tiles, groups): the same id keeps its object, in the new order.
    const out = next.map((n) => {
      const old = had.get((n as Obj).id as string);
      if (!old) return n;
      reconcileObject(old, n as Obj);
      return old;
    });
    return out.length === cur.length && out.every((o, i) => o === cur[i]) ? null : out;
  }
  if (sameJson(cur, next)) return null;
  if (cur.length === next.length) {
    for (let i = 0; i < next.length; i++) reconcile(cur, i, next[i]);
    return null;
  }
  // Rows came or went: a new array. Shifting them in place would rewrite each row with its
  // neighbour's values, and an Undo holding a removed row would put back the wrong one.
  return next;
}

/** Rows by id when every one is an object with its own string id; else null. */
function byId(list: unknown[]): Map<string, Obj> | null {
  const map = new Map<string, Obj>();
  for (const v of list) {
    if (!isObj(v) || typeof v.id !== 'string' || map.has(v.id)) return null;
    map.set(v.id, v);
  }
  return map;
}
