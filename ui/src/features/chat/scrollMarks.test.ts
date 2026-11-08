import assert from 'node:assert/strict';
import { test } from 'node:test';

import { markOf, nearest, place } from './scrollMarks.ts';

test('highlights use their own color, else their kind', () => {
  assert.deepEqual(markOf({ id: 'a', highlight: { kind: 'mention' } }), { color: '--accent-line', rank: 8 });
  assert.equal(markOf({ id: 'a', highlight: { kind: 'rule', color: '#ff0000ff' } })?.color, '#ff0000ff');
});

test('event lines get their kind color; plain lines and room info none', () => {
  assert.equal(markOf({ id: 'a', notice: { kind: 'raid' } })?.color, '--twitch');
  assert.equal(markOf({ id: 'a', notice: { kind: 'moderation' } })?.color, '--yellow');
  assert.equal(markOf({ id: 'a', notice: { kind: 'info' } }), null);
  assert.equal(markOf({ id: 'a' }), null);
});

test('a notice with a highlight color keeps that color', () => {
  assert.equal(markOf({ id: 'a', highlight: { kind: 'sub', color: '#00ff00' }, notice: { kind: 'sub' } })?.color, '#00ff00');
});

const at = (id: string, mid: number, rank = 1) => ({ id, mid, rank, color: '--red' });

test('ticks land in proportion to the content', () => {
  const t = place([at('a', 500), at('b', 1500)], 2000, 400);
  assert.deepEqual(
    t.map((x) => [x.id, x.y]),
    [
      ['a', 100],
      ['b', 300],
    ],
  );
});

test('ticks that share a spot keep the higher rank, then the newer line', () => {
  const t = place([at('old', 1000, 1), at('new', 1002, 1), at('hot', 1001, 5)], 2000, 400);
  assert.deepEqual(t.map((x) => x.id), ['hot']);
  assert.deepEqual(place([at('old', 1000), at('new', 1002)], 2000, 400).map((x) => x.id), ['new']);
});

test('important ticks come last, so they draw on top', () => {
  const t = place([at('a', 100, 8), at('b', 900, 1)], 2000, 400);
  assert.deepEqual(t.map((x) => x.id), ['b', 'a']);
});

test('nothing to place on an empty track', () => {
  assert.deepEqual(place([at('a', 1)], 0, 400), []);
  assert.deepEqual(place([at('a', 1)], 100, 0), []);
});

test('the closest tick within reach is picked', () => {
  const t = place([at('a', 500), at('b', 540)], 2000, 400);
  assert.equal(nearest(t, 101)?.id, 'a');
  assert.equal(nearest(t, 107)?.id, 'b');
  assert.equal(nearest(t, 150), null);
});
