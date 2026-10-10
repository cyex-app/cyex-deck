import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { ActivityFilters } from '../../ipc/bindings/ActivityFilters';
import type { SmActivity } from '../../ipc/bindings/SmActivity';
import { adIn, capPush, graph, shares, visibleActivity } from './logic.ts';

const ALL: ActivityFilters = { follows: true, subs: true, gifts: true, cheers: true, raids: true, redemptions: true };

function item(id: string, kind: string): SmActivity {
  return { id, kind, text: id, user: null, ts: 0 };
}

test('activity is newest first and filtered by kind', () => {
  const items = [item('a', 'follow'), item('b', 'sub'), item('c', 'cheer'), item('d', 'mystery')];
  assert.deepEqual(visibleActivity(items, ALL).map((i) => i.id), ['d', 'c', 'b', 'a']);
  const noFollows = { ...ALL, follows: false, cheers: false };
  assert.deepEqual(visibleActivity(items, noFollows).map((i) => i.id), ['d', 'b']);
  // The input stays as it was.
  assert.equal(items[0].id, 'a');
});

test('capPush keeps the newest', () => {
  assert.deepEqual(capPush([1, 2, 3], 4, 3), [2, 3, 4]);
  assert.deepEqual(capPush([], 1, 3), [1]);
});

test('graph scales into the box', () => {
  assert.equal(graph([[0, 5]], 100, 40).line, '');
  const g = graph([[0, 10], [50, 20], [100, 30]], 100, 40);
  assert.deepEqual([g.min, g.max], [10, 30]);
  const pts = g.line.split(' ').map((p) => p.split(',').map(Number));
  assert.deepEqual(pts[0], [0, 38]);
  assert.deepEqual(pts[2], [100, 2]);
  assert.ok(g.area.startsWith('M0,40 L0,38') && g.area.endsWith('L100,40 Z'));
  // Flat: the middle.
  const flat = graph([[0, 7], [10, 7]], 100, 40);
  assert.equal(flat.line, '0,20 100,20');
});

test('ad countdown', () => {
  const now = Date.parse('2026-10-10T12:00:00Z');
  assert.equal(adIn('2026-10-10T12:05:00Z', now), 300);
  assert.equal(adIn('2026-10-10T11:00:00Z', now), 0);
  assert.equal(adIn('', now), null);
  assert.equal(adIn('nope', now), null);
});

test('prediction shares', () => {
  assert.deepEqual(shares([300, 100]), [75, 25]);
  assert.deepEqual(shares([0, 0]), [0, 0]);
});
