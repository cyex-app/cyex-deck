import assert from 'node:assert/strict';
import { test } from 'node:test';

import { calloutAt, CONFIRM_MS, expiry, NOTCH, type Timers } from './confirmCallout.ts';

const btn = { left: 100, right: 122, top: 10, bottom: 32 };

test('V107-P4 the callout sits centered under its button', () => {
  const p = calloutAt(btn, 60, 1000);
  assert.deepEqual(p, { left: 81, top: 32 + NOTCH + 1, notch: 30 });
});

test('V107-P4 near the window edge it moves in and the notch stays on the button', () => {
  const edge = { left: 970, right: 992, top: 10, bottom: 32 };
  const p = calloutAt(edge, 80, 1000);
  assert.equal(p.left, 1000 - 6 - 80);
  assert.equal(p.left + p.notch, 981);
  // Never past the box's own corner.
  const far = calloutAt({ left: 996, right: 1000, top: 0, bottom: 4 }, 80, 1000);
  assert.ok(far.notch <= 80 - NOTCH - 3);
});

function fake(): Timers & { fire: () => void; cleared: number } {
  let run: (() => void) | null = null;
  const t = {
    cleared: 0,
    set: (r: () => void, ms: number) => {
      assert.equal(ms, CONFIRM_MS);
      run = r;
      return 1;
    },
    clear: () => {
      t.cleared++;
      run = null;
    },
    fire: () => run?.(),
  };
  return t;
}

test('V107-P4 the callout ends once after 3 s, not after a cancel', () => {
  let ended = 0;
  const t = fake();
  const a = expiry(() => ended++, CONFIRM_MS, t);
  t.fire();
  t.fire();
  a.cancel();
  assert.equal(ended, 1);
  assert.equal(t.cleared, 0, 'nothing left to clear');

  const b = fake();
  const c = expiry(() => ended++, CONFIRM_MS, b);
  c.cancel();
  c.cancel();
  b.fire();
  assert.equal(ended, 1);
  assert.equal(b.cleared, 1);
});
