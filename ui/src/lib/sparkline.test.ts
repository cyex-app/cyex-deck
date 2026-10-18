import assert from 'node:assert/strict';
import { test } from 'node:test';

import { BAND_SHARE, graph, inBand, nearest, peakOf, pointAt } from './sparkline.ts';

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

test('from moves the first sample right by the band', () => {
  const g = graph([[50, 10], [100, 20]], 100, 40, 0);
  assert.equal(g.x0, 12);
  assert.ok(g.line.startsWith('12,'));
  assert.ok(g.area.startsWith('M12,40'));
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

test('pointAt matches the polyline', () => {
  const s: [number, number][] = [[0, 10], [50, 20], [100, 30]];
  assert.deepEqual(pointAt(s, 2, 100, 40), [100, 2]);
  assert.deepEqual(pointAt(s, 0, 100, 40), [0, 38]);
});

test('before band is a fixed small share, points use the rest', () => {
  // Live 2 h, watched the last 60 s: band is 12 %, points span the other 88 %.
  const s: [number, number][] = [[7140, 5], [7200, 9]];
  const g = graph(s, 100, 40, 0);
  assert.equal(g.x0, BAND_SHARE * 100);
  assert.ok(g.line.endsWith('100,2'));
  // Hover across the watched part picks the nearest point, not always the first.
  assert.equal(nearest(s, 20, 100, 0), 0);
  assert.equal(nearest(s, 70, 100, 0), 1);
  assert.deepEqual(pointAt(s, 1, 100, 40, 0), [100, 2]);
});

test('small gap gets a smaller band, no gap none', () => {
  // Gap 6 s of 66 s total: about 9 %, under the cap.
  const small = graph([[6, 1], [66, 2]], 100, 40, 0);
  assert.equal(small.x0, 9.1);
  assert.equal(graph([[0, 1], [60, 2]], 100, 40, 0).x0, 0);
  assert.equal(graph([[0, 1], [60, 2]], 100, 40).x0, 0);
});

test('peak tie picks the earliest', () => {
  assert.deepEqual(peakOf([[1, 5], [2, 9], [3, 9], [4, 2]]), [2, 9]);
  assert.equal(peakOf([]), null);
});

test('band hit test', () => {
  assert.equal(inBand(10, 34), true);
  assert.equal(inBand(34, 34), false);
  assert.equal(inBand(0, 0), false);
});
