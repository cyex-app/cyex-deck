// V107-R33 Where the mouse's back and forward buttons go: the tabs and views shown before,
// like a browser's history. Places that are gone by then (a closed or popped-out tab) are
// skipped.

const MAX = 50;

export class PlaceHistory<P> {
  private back: P[] = [];
  private forward: P[] = [];
  private readonly same: (a: P, b: P) => boolean;

  constructor(same: (a: P, b: P) => boolean) {
    this.same = same;
  }

  /** Leaving `from` for somewhere new: it goes on the back list, and forward is cleared. */
  left(from: P): void {
    const last = this.back.at(-1);
    if (last === undefined || !this.same(last, from)) this.back.push(from);
    if (this.back.length > MAX) this.back.shift();
    this.forward = [];
  }

  /** The place to go back to from `here`, skipping ones `ok` refuses; `here` goes forward. */
  goBack(here: P | null, ok: (p: P) => boolean): P | null {
    return step(this.back, this.forward, here, ok, this.same);
  }

  /** The place to go forward to from `here`; `here` goes back. */
  goForward(here: P | null, ok: (p: P) => boolean): P | null {
    return step(this.forward, this.back, here, ok, this.same);
  }
}

function step<P>(from: P[], to: P[], here: P | null, ok: (p: P) => boolean, same: (a: P, b: P) => boolean): P | null {
  while (from.length) {
    const p = from.pop() as P;
    if (!ok(p) || (here !== null && same(p, here))) continue;
    if (here !== null) to.push(here);
    return p;
  }
  return null;
}
