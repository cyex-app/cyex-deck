import assert from 'node:assert/strict';
import { test } from 'node:test';

import { groupAt, zoneAt } from './zones.ts';

const box = { left: 100, top: 100, width: 200, height: 100 };

test('the middle of a tile swaps (U3)', () => {
  assert.equal(zoneAt(box, 200, 150), 'center');
});

test('edges of a tile put the dragged one on that side (U3)', () => {
  assert.equal(zoneAt(box, 105, 150), 'left');
  assert.equal(zoneAt(box, 295, 150), 'right');
  assert.equal(zoneAt(box, 200, 103), 'top');
  assert.equal(zoneAt(box, 200, 198), 'bottom');
  // Near a corner, the closer edge wins.
  assert.equal(zoneAt(box, 104, 120), 'left');
  assert.equal(zoneAt(box, 130, 102), 'top');
});

const g = (group: string | null) => ({ group });

test('a tab dropped inside a group joins it (U5)', () => {
  assert.equal(groupAt(g('a'), g('a'), null), 'a');
  assert.equal(groupAt(g('a'), g('a'), 'b'), 'a');
});

test('a tab dropped at its own group’s edge stays in it', () => {
  assert.equal(groupAt(g('a'), g(null), 'a'), 'a');
  assert.equal(groupAt(undefined, g('a'), 'a'), 'a');
});

test('anywhere else, a tab leaves its group', () => {
  assert.equal(groupAt(g('a'), g(null), null), null);
  assert.equal(groupAt(g('a'), g('b'), 'c'), null);
  assert.equal(groupAt(undefined, undefined, 'a'), null);
});
