import assert from 'node:assert/strict';
import { test } from 'node:test';

import { place, point } from './place.ts';

const view = { width: 800, height: 600 };
const tip = { width: 120, height: 150 };
const box = (left: number, top: number, w = 28, h = 28) => ({ left, top, right: left + w, bottom: top + h });

test('a tooltip goes under its anchor when there is room', () => {
  const p = place(box(300, 100), tip, view, { align: 'center' });
  assert.deepEqual(p, { left: 254, top: 134, side: 'below' });
});

test('near the bottom it flips above instead of leaving the window', () => {
  // An emote in the last chat line: 150 px under it would end at 728.
  const p = place(box(300, 550), tip, view, { align: 'center' });
  assert.equal(p.side, 'above');
  assert.equal(p.top, 550 - 6 - 150);
});

test('near the top a tooltip that prefers above goes below', () => {
  const p = place(box(300, 20), tip, view, { side: 'above', align: 'center' });
  assert.equal(p.side, 'below');
  assert.equal(p.top, 54);
});

test('no room on either side: it stays inside, even over the anchor', () => {
  const tall = { width: 120, height: 580 };
  const p = place(box(300, 280), tall, view);
  assert.equal(p.top, 12);
  assert.ok(p.top + tall.height <= 600 - 8);
});

test('sideways it is pushed back in from the left and right edges', () => {
  assert.equal(place(box(2, 100), tip, view, { align: 'center' }).left, 8);
  assert.equal(place(box(790, 100, 8), tip, view, { align: 'center' }).left, 800 - 8 - 120);
});

test('a card beside an emote opens left when the right side is full', () => {
  const card = { width: 300, height: 200 };
  assert.deepEqual(place(box(100, 100), card, view, { side: 'right', gap: 8 }), { left: 136, top: 100, side: 'right' });
  const p = place(box(600, 100), card, view, { side: 'right', gap: 8 });
  assert.equal(p.side, 'left');
  assert.equal(p.left, 600 - 8 - 300);
  // Low in the window, its bottom lines up with the emote's so all of it shows.
  assert.equal(place(box(100, 500), card, view, { side: 'right', gap: 8 }).top, 528 - 200);
});

test('a menu at the pointer opens up and to the left near the bottom-right corner', () => {
  const menu = { width: 200, height: 300 };
  assert.deepEqual(place(point(100, 100), menu, view, { gap: 0 }), { left: 100, top: 100, side: 'below' });
  assert.deepEqual(place(point(700, 500), menu, view, { gap: 0 }), { left: 500, top: 200, side: 'above' });
});

test('an end-aligned dropdown near the left edge lines up with the start instead', () => {
  const drop = { width: 220, height: 100 };
  const p = place(box(20, 100, 40, 30), drop, view, { align: 'end' });
  assert.equal(p.left, 20);
});

test('bigger than the window: the top-left corner stays visible', () => {
  const p = place(box(10, 10), { width: 900, height: 700 }, view);
  assert.deepEqual([p.left, p.top], [8, 8]);
});
