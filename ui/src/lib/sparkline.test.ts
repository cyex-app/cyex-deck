import assert from 'node:assert/strict';
import { test } from 'node:test';

import { graph, nearest, place } from './sparkline.ts';

test('graph scales into the box', () => {
  assert.equal(graph([[0, 5]], 100, 40).line, '');
  assert.equal(graph([], 100, 40).line, '');
  const g = graph([[0, 10], [50, 20], [100, 30]], 100, 40);
  assert.deepEqual([g.min, g.max], [10, 30]);
  const pts = g.line.split(' ').map((p) => p.split(',').map(Number));
  assert.deepEqual(pts[0], [0, 38]);
  assert.deepEqual(pts[2], [100, 2]);
  assert.ok(g.area.startsWith('M0,40 L0,38') && g.area.endsWith('L100,40 Z'));
});

test('flat line sits in the middle', () => {
  assert.equal(graph([[0, 7], [10, 7]], 100, 40).line, '0,20 100,20');
});

test('from moves the first sample right', () => {
  const g = graph([[50, 10], [100, 20]], 100, 40, 0);
  assert.equal(g.x0, 50);
  assert.ok(g.line.startsWith('50,'));
  assert.ok(g.area.startsWith('M50,40'));
});

test('small values keep 0 as the floor', () => {
  // min 10 is under 30 % of 100: 10 sits just above the bottom, not on it.
  const g = graph([[0, 10], [10, 100]], 100, 40);
  const first = g.line.split(' ')[0].split(',').map(Number);
  assert.ok(first[1] < 38);
  // min 90 of 100 keeps the tight range: the low end hits the floor.
  const tight = graph([[0, 90], [10, 100]], 100, 40);
  assert.deepEqual(tight.line.split(' ')[0].split(',').map(Number), [0, 38]);
});

test('nearest picks the closest sample', () => {
  const s: [number, number][] = [[0, 1], [50, 2], [100, 3]];
  assert.equal(nearest(s, -20, 100), 0);
  assert.equal(nearest(s, 24, 100), 0);
  assert.equal(nearest(s, 26, 100), 1);
  assert.equal(nearest(s, 500, 100), 2);
  assert.equal(nearest([], 5, 100), -1);
  assert.equal(nearest([[3, 3]], 5, 100), 0);
  // With an axis start the samples sit further right.
  assert.equal(nearest([[50, 1], [100, 2]], 10, 100, 0), 0);
});

test('place matches the polyline', () => {
  const s: [number, number][] = [[0, 10], [50, 20], [100, 30]];
  assert.deepEqual(place(s, 2, 100, 40), [100, 2]);
  assert.deepEqual(place(s, 0, 100, 40), [0, 38]);
});
