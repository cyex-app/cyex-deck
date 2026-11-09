import assert from 'node:assert/strict';
import { test } from 'node:test';

import { trackPointer } from './trackPointer.ts';

function stub() {
  const on = new Map<string, Set<(e: Event) => void>>();
  return {
    on,
    count: () => [...on.values()].reduce((n, s) => n + s.size, 0),
    addEventListener: (t: string, f: (e: Event) => void) => void (on.get(t) ?? on.set(t, new Set()).get(t)!).add(f),
    removeEventListener: (t: string, f: (e: Event) => void) => void on.get(t)?.delete(f),
    fire: (t: string) => [...(on.get(t) ?? [])].forEach((f) => f(new Event(t))),
  };
}

test('moves are passed on until the press ends', () => {
  const w = stub();
  let moves = 0;
  trackPointer(() => moves++, () => {}, w);
  w.fire('pointermove');
  w.fire('pointermove');
  assert.equal(moves, 2);
});

for (const kind of ['pointerup', 'pointercancel', 'blur']) {
  test(`${kind} ends once and leaves no listeners`, () => {
    const w = stub();
    const ended: string[] = [];
    trackPointer(() => {}, (e) => ended.push(e.type), w);
    assert.equal(w.count(), 4);
    w.fire(kind);
    w.fire(kind);
    assert.deepEqual(ended, [kind]);
    assert.equal(w.count(), 0);
  });
}

test('stop removes everything without calling onend, and twice is fine', () => {
  const w = stub();
  let ended = 0;
  const stop = trackPointer(() => {}, () => ended++, w);
  stop();
  stop();
  w.fire('pointerup');
  assert.equal(w.count(), 0);
  assert.equal(ended, 0);
});
