// V101-01 Alternating line colors. A line gets its stripe when it first shows, the
// opposite of the neighbour it lands next to, and keeps it. Lines leaving the top,
// lines arriving at either end and a paused list catching up never flip the others.
// (A positional `:nth-of-type` rule renumbered every line each time the oldest left.)

/** Stripe per line id, `true` for the tinted one. `known` is the previous result. */
export function stripes(ids: readonly string[], known: ReadonlyMap<string, boolean>): Map<string, boolean> {
  const out = new Map<string, boolean>();
  const at = ids.findIndex((id) => known.has(id));
  if (at === -1) {
    ids.forEach((id, i) => out.set(id, i % 2 === 1));
    return out;
  }
  const first = known.get(ids[at])!;
  out.set(ids[at], first);
  let v = first;
  for (let i = at + 1; i < ids.length; i++) {
    v = known.get(ids[i]) ?? !v;
    out.set(ids[i], v);
  }
  // Everything above the first known line is new: history loaded at the top.
  v = first;
  for (let i = at - 1; i >= 0; i--) {
    v = !v;
    out.set(ids[i], v);
  }
  return out;
}
