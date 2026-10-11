import assert from 'node:assert/strict';
import { test } from 'node:test';

import { edgeAt, groupAt, rowSlot, sideAfterDrop, zoneAt } from './zones.ts';

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

// Two rows of a wrapped bar (V101-38): a, b on top, c below with room after it.
const cell = (id: string, left: number, top: number) => ({ id, rect: { left, top, width: 100, height: 34 } });
const cells = [cell('a', 0, 0), cell('b', 102, 0), cell('c', 0, 34)];

test('a tab dropped in the gap between two tabs lands between them', () => {
  assert.deepEqual(rowSlot(cells, 101, 10), { id: 'b', after: false });
  assert.deepEqual(rowSlot(cells, 30, 10), { id: 'a', after: false });
});

test('past a row’s last tab, it goes after that tab, not to the end of the bar', () => {
  assert.deepEqual(rowSlot(cells, 400, 10), { id: 'b', after: true });
  assert.deepEqual(rowSlot(cells, 400, 50), { id: 'c', after: true });
});

test('below every row it is the end of the bar', () => {
  assert.equal(rowSlot(cells, 50, 90), null);
});

test('a panel dropped on its partner lands on the closest edge (V102-23)', () => {
  assert.equal(edgeAt(box, 200, 110), 'top');
  assert.equal(edgeAt(box, 290, 150), 'right');
  assert.equal(edgeAt(box, 200, 195), 'bottom');
  assert.equal(sideAfterDrop('chat', 'bottom'), 'bottom', 'chat dropped under the stream');
  assert.equal(sideAfterDrop('stream', 'bottom'), 'top', 'stream dropped under the chat');
  assert.equal(sideAfterDrop('stream', 'left'), 'right');
});
