import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  aim,
  atBottom,
  carry,
  clampTop,
  easeOutCubic,
  frame,
  type Glide,
  moves,
  NOTCH_PX,
  notches,
  positionAt,
  SCROLL_MS,
} from './scrollMath.ts';

const MAX = 1000;

test("Chatterino's curve: OutCubic over 150 ms (V103-11)", () => {
  assert.equal(easeOutCubic(0), 0);
  assert.equal(easeOutCubic(1), 1);
  assert.equal(easeOutCubic(0.5), 0.875);
  const g: Glide = { from: 100, to: 280, start: 0, bottom: false };
  assert.equal(positionAt(g, 0), 100);
  assert.equal(positionAt(g, SCROLL_MS), 280);
  assert.equal(positionAt(g, SCROLL_MS * 4), 280);
  assert.equal(positionAt(g, -20), 100);
  let last = 100;
  for (let t = 0; t <= SCROLL_MS; t += 5) {
    const at = positionAt(g, t);
    assert.ok(at >= last && at <= 280, 'never backs up or overshoots');
    last = at;
  }
});

test('a notch is 180 px; touchpads and free-spinning wheels stay native', () => {
  assert.equal(NOTCH_PX, 180);
  assert.equal(notches(-120, 100, 0), 1);
  assert.equal(notches(240, -200, 0), -2);
  assert.equal(notches(30, -25, 0), 0);
  assert.equal(notches(-7, 4, 0), 0);
  assert.equal(notches(undefined, 3, 1), 1);
  assert.equal(notches(undefined, 100, 0), 0);
  assert.equal(notches(-120, 1, 2), 0);
  assert.equal(notches(-120, -100, 0), 0, 'the two deltas disagree');
  assert.equal(notches(0, 0, 0), 0);
});

test('a notch from rest glides from the view to 180 px further', () => {
  assert.deepEqual(aim(null, 200, NOTCH_PX, MAX, 10), { from: 200, to: 380, start: 10, bottom: false });
});

test('retargeting: notches during a glide add to its target, the curve restarts from the view', () => {
  const g = aim(null, 0, NOTCH_PX, MAX, 0)!;
  const view = positionAt(g, 50);
  const again = aim(g, view, NOTCH_PX, MAX, 50)!;
  assert.deepEqual(again, { from: view, to: 360, start: 50, bottom: false });
  const back = aim(again, positionAt(again, 60), -NOTCH_PX, MAX, 60)!;
  assert.equal(back.to, 180, 'turning back counts from the target too');
  assert.equal(back.from, positionAt(again, 60));
});

test('clamping: targets stay within the area; no room is no glide', () => {
  assert.equal(aim(null, 900, NOTCH_PX, MAX, 0)?.to, MAX);
  assert.equal(aim(null, 90, -NOTCH_PX, MAX, 0)?.to, 0);
  assert.equal(aim(null, MAX, NOTCH_PX, MAX, 0), null);
  assert.equal(aim(null, 0, -NOTCH_PX, MAX, 0), null);
  assert.equal(aim(null, 0, NOTCH_PX, 0, 0), null);
  const toEnd = aim(null, 850, NOTCH_PX, MAX, 0)!;
  assert.equal(aim(toEnd, 900, NOTCH_PX, MAX, 20), null, 'the target is already at the end');
  assert.equal(clampTop(50, -10), 0);
  assert.equal(clampTop(-5, MAX), 0);
  assert.equal(clampTop(1200, MAX), MAX);
  assert.equal(frame({ from: 0, to: 900, start: 0, bottom: false }, SCROLL_MS, 600).top, 600, 'content shrank');
  assert.ok(moves(0, 2, MAX) && !moves(0, 1, MAX) && !moves(MAX, 5, MAX));
});

test('at-bottom rule: decided by the target, which then keeps to the bottom as content grows', () => {
  assert.ok(atBottom(999.5, MAX));
  assert.ok(!atBottom(990, MAX));
  assert.ok(atBottom(0, -20), 'nothing to scroll is the bottom');
  const down = aim(null, 850, NOTCH_PX, MAX, 0)!;
  assert.equal(down.bottom, true);
  assert.equal(carry(down, 0, 1300).to, 1300, 'new lines came in: still headed for the bottom');
  const up = aim(null, MAX, -NOTCH_PX, MAX, 0)!;
  assert.equal(up.bottom, false);
  assert.equal(carry(up, 0, 1300).to, 820, 'scrolled up: growth below changes nothing');
});

test('moved by something else: the glide carries on from there by the same amount', () => {
  const g: Glide = { from: 500, to: 680, start: 0, bottom: false };
  assert.deepEqual(carry(g, -40, MAX), { from: 460, to: 640, start: 0, bottom: false });
  assert.deepEqual(carry(g, 0.6, MAX), g, 'rounding is not a move');
  const jumped = carry(g, 400, MAX);
  assert.equal(jumped.to, MAX);
  assert.equal(jumped.bottom, true, 'a jump past the end lands on the bottom');
});

test('frames end after 150 ms', () => {
  const g: Glide = { from: 0, to: 180, start: 100, bottom: false };
  assert.equal(frame(g, 100 + SCROLL_MS - 1, MAX).done, false);
  assert.deepEqual(frame(g, 100 + SCROLL_MS, MAX), { top: 180, done: true });
});
