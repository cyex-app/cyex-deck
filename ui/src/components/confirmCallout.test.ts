import assert from 'node:assert/strict';
import { test } from 'node:test';

import { bubbleAt, calloutAt, CONFIRM_MS, expiry, NOTCH, type Timers } from './confirmCallout.ts';

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

// V108-C21
const clear = (b: { left: number; top: number }, w: number, h: number, a: { left: number; right: number; top: number; bottom: number }) =>
  b.left + w <= a.left || b.left >= a.right || b.top + h <= a.top || b.top >= a.bottom;

test('V108-C21 the bubble never covers the control, wherever it is', () => {
  const view = { w: 1000, h: 700 };
  const spots = [
    { left: 100, right: 160, top: 100, bottom: 127 },
    { left: 940, right: 1000, top: 300, bottom: 327 },
    { left: 0, right: 40, top: 670, bottom: 697 },
    { left: 500, right: 900, top: 0, bottom: 27 },
  ];
  for (const a of spots) {
    for (const px of [undefined, a.left, (a.left + a.right) / 2, a.right]) {
      const b = bubbleAt(a, 90, 24, view.w, view.h, 6, px);
      assert.ok(clear(b, 90, 24, a), `clear of ${JSON.stringify(a)} at ${px}`);
      assert.ok(b.left >= 6 && b.left + 90 <= view.w - 6);
    }
  }
});

test('V108-C21 under the control, above it when the window ends', () => {
  const low = bubbleAt({ left: 100, right: 160, top: 660, bottom: 687 }, 90, 24, 1000, 700);
  assert.equal(low.below, false);
  assert.equal(low.top, 660 - NOTCH - 1 - 24);
  const mid = bubbleAt(btn, 90, 24, 1000, 700);
  assert.equal(mid.below, true);
  assert.equal(mid.top, 32 + NOTCH + 1);
});

test('V108-C21 the notch points at the press point, kept on the control', () => {
  const a = { left: 100, right: 160, top: 10, bottom: 37 };
  const at = bubbleAt(a, 90, 24, 1000, 700, 6, 110);
  assert.equal(at.left + at.notch, 110);
  const out = bubbleAt(a, 90, 24, 1000, 700, 6, 400);
  assert.equal(out.left + out.notch, 160, 'clamped to the control');
});

test('V108-C21 pausing keeps the time left, resuming runs only that', () => {
  let clock = 0;
  const sets: { run: () => void; ms: number; live: boolean }[] = [];
  const t: Timers = {
    now: () => clock,
    set: (run, ms) => (sets.push({ run, ms, live: true }), sets.length - 1),
    clear: (id) => void (sets[id as number].live = false),
  };
  let ended = 0;
  const e = expiry(() => ended++, 3000, t);
  clock = 1000;
  e.pause();
  assert.equal(sets[0].live, false);
  clock = 9000;
  e.resume();
  assert.equal(sets[1].ms, 2000, 'the time that was left');
  e.pause();
  e.pause();
  e.resume();
  e.resume();
  assert.equal(sets.length, 3, 'double pause/resume do not stack');
  sets[2].run();
  assert.equal(ended, 1);
  e.resume();
  assert.equal(sets.length, 3, 'nothing after it ended');
});
